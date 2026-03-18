use crate::application::dto::apperror::AppError;
use crate::application::dto::server_dto::{ServerItem, MemberItem}; // Ajoutez MemberItem
use crate::application::dto::channel_dto::ChannelItem;
use crate::application::dto::auth_dto::BanInfo;
use crate::domain::models::{channel, refresh_token, server_member, user, server_model, server_ban};
use crate::domain::models::user::UserStatus;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use chrono::{Duration, Utc};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, ModelTrait, QueryOrder};
use uuid::Uuid;
use crate::application::dto::auth_dto::{LoginRequest, LoginResponse, LogoutResponse, MeResponse, RefreshRequest, RefreshResponse, SignupRequest, SignupResponse};
use crate::application::utils::jwt::{ generate_token };
use crate::application::dto::token_dto::Claims;
use tokio::sync::broadcast;
use serde_json::json;
use std::io::Cursor;

pub async fn register_user(db: &DatabaseConnection, req: SignupRequest) -> Result<SignupResponse, AppError> {
    if req.password.len() < 8 {
        return Err(AppError::BadRequest("Password too short".to_string()));
    }
    
    let user = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if user.is_some() {
        return Err(AppError::BadRequest("Username already in use".to_string()));
    }

    let salt_str = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();

    let hashed_password = argon
        .hash_password(req.password.as_bytes(), &salt_str)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .to_string();

    let user_id = Uuid::new_v4();

    let new_user = user::ActiveModel {
        id: Set(user_id),
        username: Set(req.username.clone()),
        password_hash: Set(hashed_password),
        status: Set(UserStatus::Offline),
        display_name: Set(None),
        avatar_url: Set(None),
        ..Default::default()
    };

    new_user.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    return Ok(SignupResponse{
        id: user_id,
        username: req.username,
    });
}

pub async fn login_user(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    req: LoginRequest
) -> Result<LoginResponse, AppError> {
    let user = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized("Invalid username or password".to_string()))?;

    // --- MISE A JOUR DB : ONLINE ---
    let mut user_active: user::ActiveModel = user.clone().into();
    user_active.status = Set(UserStatus::Online);
    let _ = user_active.update(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;
    // -------------------------------

    // --- NOUVEAU : BROADCAST "ONLINE" ---
    // 1. Récupérer les serveurs de l'utilisateur pour savoir à qui envoyer
    let memberships_result = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(user.id))
        .all(db)
        .await;

    // 2. Diffuser le statut update si la récup fonctionne
    if let Ok(memberships) = memberships_result {
        for member in memberships {
            let payload = json!({
                "type": "user_status_change",
                "data": {
                    "server_id": member.server_id,
                    "user_id": user.id,
                    "status": "Online"
                }
            });
            // On ignore les erreurs s'il n'y a personne pour écouter
            let _ = tx.send(payload.to_string());
        }
    }
    // ------------------------------------

    let existing_token = refresh_token::Entity::find()
        .filter(refresh_token::Column::UserId.eq(user.id))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if let Some(token_model) = existing_token {
        if token_model.expires_at > Utc::now() {
            return Ok(LoginResponse { 
                access_token: token_model.token, 
                id: user.id, 
                username: user.username 
            });
        } else {
            token_model.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;
        }
    }

    let new_token_string = generate_token(user.id, user.username.clone())?;
    let expires_at = Utc::now() + Duration::hours(48);

    let session_token = refresh_token::ActiveModel {
        id: Set(Uuid::new_v4()),
        token: Set(new_token_string.clone()),
        user_id: Set(user.id),
        expires_at: Set(expires_at.into()),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };

    session_token.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(LoginResponse { 
        access_token: new_token_string, 
        id: user.id, 
        username: user.username 
    })
}

pub async fn logout_user(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims
) -> Result<LogoutResponse, AppError> {
    
    // 1. Mise à jour DB (Offline)
    let user_update = user::ActiveModel {
        id: Set(claims.sub),
        status: Set(UserStatus::Offline),
        ..Default::default()
    };
    
    if let Err(e) = user_update.update(db).await {
         println!("Error setting user offline: {}", e);
    }

    // 2. BROADCAST "user_status_change" (OFFLINE)
    // On récupère tous les serveurs où l'utilisateur est présent
    let memberships = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(claims.sub))
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // Pour chaque serveur, on notifie que l'user est passé Offline
    for member in memberships {
        let payload = json!({
            "type": "user_status_change",
            "data": {
                "server_id": member.server_id,
                "user_id": claims.sub,
                "status": "Offline"
            }
        });
        // On ignore les erreurs d'envoi (si personne n'écoute)
        let _ = tx.send(payload.to_string());
    }

    // 3. Suppression Token
    refresh_token::Entity::delete_many()
        .filter(refresh_token::Column::UserId.eq(claims.sub))
        .exec(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(LogoutResponse {
        message: "Logged out successfully".to_string()
    })
}

pub async fn refresh_access_token(db: &DatabaseConnection, req: RefreshRequest) -> Result<RefreshResponse, AppError> {
    // Vérifier que le refresh token existe et n'est pas expiré
    let refresh_token = refresh_token::Entity::find()
        .filter(refresh_token::Column::Token.eq(&req.refresh_token))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Unauthorized("Invalid refresh token".to_string()))?;

    if refresh_token.expires_at < Utc::now() {
        return Err(AppError::Unauthorized("Refresh token expired".to_string()));
    }

    // Récupérer l'utilisateur
    let user = user::Entity::find_by_id(refresh_token.user_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    // Générer un nouveau access token
    let access_token = generate_token(user.id, user.username)?;

    Ok(RefreshResponse { access_token })
}

pub async fn me(db: &DatabaseConnection, claims: Claims) -> Result<MeResponse, AppError> {
    // 1. Récupérer l'utilisateur
    let user = user::Entity::find_by_id(claims.sub)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("User not found".to_string()))?;
    
    // 2. Récupérer les serveurs de l'utilisateur
    let members_with_servers = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(user.id))
        .find_also_related(server_model::Entity)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // 3. Collecter les ID des serveurs
    let server_ids: Vec<i32> = members_with_servers.iter()
        .filter_map(|(_, s)| s.as_ref().map(|srv| srv.id))
        .collect();

    // 4. Fetch Channels (existants)
    let all_channels = channel::Entity::find()
        .filter(channel::Column::ServerId.is_in(server_ids.clone()))
        .order_by_asc(channel::Column::Position)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // 5. Fetch Members (NOUVEAU) : On récupère tous les membres de ces serveurs + infos user
    let all_members = server_member::Entity::find()
        .filter(server_member::Column::ServerId.is_in(server_ids))
        .find_also_related(user::Entity) // Join pour avoir username/avatar
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // 7. Fetch active bans for this user (bans that happened while offline)
    let raw_bans = server_ban::Entity::find()
        .filter(server_ban::Column::UserId.eq(user.id))
        .find_also_related(server_model::Entity)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let now = chrono::Utc::now().naive_utc();
    let pending_bans: Vec<BanInfo> = raw_bans
        .into_iter()
        .filter(|(ban, _)| ban.banned_until.is_none() || ban.banned_until.map(|t| t > now).unwrap_or(false))
        .filter_map(|(ban, server_opt)| {
            server_opt.map(|s| BanInfo {
                server_id: s.id,
                server_name: s.name,
                banned_until: ban.banned_until.map(|t| t.to_string()),
            })
        })
        .collect();

    // 8. Assemblage
    let servers: Vec<ServerItem> = members_with_servers
        .into_iter()
        .filter_map(|(_member, server)| {
            server.map(|s| {
                // Filtrer les channels pour ce serveur
                let server_channels = all_channels.iter()
                    .filter(|c| c.server_id == s.id)
                    .map(|c| ChannelItem {
                        id: c.id,
                        server_id: c.server_id,
                        name: c.name.clone(),
                        description: c.description.clone(),
                        position: c.position,
                    })
                    .collect();

                // Filtrer les membres pour ce serveur
                let server_members = all_members.iter()
                    .filter(|(m, _)| m.server_id == s.id)
                    .filter_map(|(m, u_opt)| {
                        u_opt.as_ref().map(|u| MemberItem {
                            id: m.id,
                            user_id: u.id,
                            username: u.username.clone(),
                            display_name: u.display_name.clone(),
                            avatar_url: u.avatar_url.clone(),
                            role: format!("{:?}", m.role), 
                            status: format!("{:?}", u.status),
                        })
                    })
                    .collect();

                // --- AJOUT : Calcul des admins pour ce serveur ---
                let admins: Vec<Uuid> = all_members.iter()
                    .filter(|(m, _)| m.server_id == s.id && m.role == server_member::MemberRole::Admin)
                    .map(|(m, _)| m.user_id)
                    .collect();
                // ------------------------------------------------

                ServerItem {
                    id: s.id,
                    name: s.name,
                    description: s.description,
                    icon_url: s.icon_url,
                    owner_id: s.owner_id,
                    admins, // <--- AJOUT DU CHAMP MANQUANT
                    invitcode: s.invitcode,
                    channels: server_channels,
                    members: server_members,
                }
            })
        })
        .collect();

    Ok(MeResponse {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        servers,
        pending_bans,
    })
}

pub async fn update_user_status(
    db: &DatabaseConnection,
    tx: &broadcast::Sender<String>,
    user_id: Uuid,
    new_status: UserStatus,
) -> Result<(), AppError> {
    let user = user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    let mut active_user: user::ActiveModel = user.into();
    active_user.status = Set(new_status.clone());
    active_user
        .update(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let broadcast_status = match new_status {
        UserStatus::Online => "online",
        UserStatus::Invisible => "invisible",
        UserStatus::Offline => "offline",
    };

    let memberships = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    for member in memberships {
        let payload = json!({
            "type": "user_status_change",
            "data": {
                "server_id": member.server_id,
                "user_id": user_id,
                "status": broadcast_status
            }
        });
        let _ = tx.send(payload.to_string());
    }

    Ok(())
}

pub async fn update_avatar(
    db: &DatabaseConnection,
    tx: &broadcast::Sender<String>,
    user_id: Uuid,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    // Validate image
    if image_data.is_empty() || image_data.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest("Image size invalid (max 5MB)".to_string()));
    }

    // Load and convert to PNG
    let img = image::load_from_memory(&image_data)
        .map_err(|_| AppError::BadRequest("Invalid image format".to_string()))?;

    // Resize if needed to keep it reasonable (max 512x512)
    let img = if img.width() > 512 || img.height() > 512 {
        img.thumbnail(512, 512)
    } else {
        img
    };

    // Generate unique filename
    let filename = format!("avatar_{}.png", Uuid::new_v4());
    let file_path = format!("./uploads/avatars/{}", filename);

    // Create uploads directory if it doesn't exist
    let dir_path = "./uploads/avatars";
    tokio::fs::create_dir_all(dir_path)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create directory: {}", e)))?;

    // Convert to PNG and save
    let png_data = Vec::new();
    let mut cursor = Cursor::new(png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| AppError::InternalServerError(format!("Failed to convert image to PNG: {}", e)))?;
    let png_data = cursor.into_inner();

    tokio::fs::write(&file_path, &png_data)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to save file: {}", e)))?;

    // Construct the URL
    let avatar_url = format!("/uploads/avatars/{}", filename);

    // Update user in database
    let user = user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    let mut active_user: user::ActiveModel = user.into();
    active_user.avatar_url = Set(Some(avatar_url.clone()));
    active_user
        .update(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // Broadcast the avatar change to all servers where user is a member
    let memberships = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    for member in memberships {
        let payload = json!({
            "type": "user_avatar_change",
            "data": {
                "server_id": member.server_id,
                "user_id": user_id,
                "avatar_url": avatar_url
            }
        });
        let _ = tx.send(payload.to_string());
    }

    Ok(avatar_url)
}