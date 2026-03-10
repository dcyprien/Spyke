use crate::application::dto::apperror::AppError;
use crate::domain::models::{channel, server_member, server_model, user, server_ban};
use crate::domain::models::server_member::MemberRole;
use crate::application::dto::channel_dto::ChannelItem;
use crate::application::dto::server_dto::{CreateChannelRequest, CreateServerRequest, BanUserRequest, CreateServerResponse, GetChannelsResponse, GetServerIdResponse, GetServerMemberResponse, GetServerResponse, JoinServerRequest, JoinServerResponse, MemberItem, ServerItem, UpdateMemberRequest, UpdateMemberResponse, UpdateServerRequest, UpdateServerResponse};
use crate::application::dto::token_dto::Claims;
use axum::http::StatusCode;
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, ActiveModelTrait, EntityTrait, ColumnTrait, QueryFilter, QueryOrder};
use uuid::Uuid;
use tokio::sync::broadcast;
use serde_json::json;

pub async fn create_server(db: &DatabaseConnection, claims: Claims, req: CreateServerRequest) -> Result<CreateServerResponse, AppError> {
    let server_check = server_model::Entity::find()
        .filter(server_model::Column::Name.eq(req.name.clone()))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if server_check.is_some() {
        return Err(AppError::BadRequest("Server's name already in use".to_string()))
    }

    if req.name.trim().is_empty() { return Err(AppError::BadRequest("Server name cannot be empty".to_string())); }
    if req.name.len() > 20 { return Err(AppError::BadRequest("Server name too long".to_string())); }
    if req.description.trim().is_empty() { return Err(AppError::BadRequest("Description required".to_string())); }
    
    let invit = (Uuid::new_v4().as_u128() % 10000) as i32;
    
    let new_server = server_model::ActiveModel {
        name: Set(req.name.clone()),
        description: Set(req.description.clone()),
        icon_url: Set(req.icon_url.clone()),
        owner_id: Set(claims.sub),
        invitcode: Set(invit),
        ..Default::default()
    };
    
    let server = new_server.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;
    
    let owner_membership = server_member::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(server.id),
        user_id: Set(claims.sub),
        role: Set(MemberRole::Owner),
        ..Default::default()
    };
    
    owner_membership.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;
    
    // Création et récupération du Channel par défaut
    let default_channel = channel::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(server.id),
        name: Set("general".to_string()),
        description: Set("General discussion".to_string()),
        position: Set(0),
    };

    let created_channel = default_channel.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // Conversion en ChannelItem pour la réponse
    let channel_item = ChannelItem {
        id: created_channel.id,
        server_id: created_channel.server_id,
        name: created_channel.name,
        description: created_channel.description,
        position: created_channel.position,
    };

    Ok(CreateServerResponse {
        id: server.id,
        name: server.name,
        invitcode: invit,
        description: server.description,
        icon_url: server.icon_url,
        owner_id: server.owner_id,
        channels: vec![channel_item],
    })
}

pub async fn get_servers(db: &DatabaseConnection, claims: Claims) -> Result<GetServerResponse, AppError> {
    let memberships = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(claims.sub))
        .find_also_related(server_model::Entity)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let server_ids: Vec<i32> = memberships.iter()
        .filter_map(|(_, s)| s.as_ref().map(|x| x.id))
        .collect();

    let all_channels = channel::Entity::find()
        .filter(channel::Column::ServerId.is_in(server_ids.clone()))
        .order_by_asc(channel::Column::Position)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let all_members = server_member::Entity::find()
        .filter(server_member::Column::ServerId.is_in(server_ids))
        .find_also_related(user::Entity)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let servers: Vec<ServerItem> = memberships
        .into_iter()
        .filter_map(|(_, server_opt)| {
            server_opt.map(|server| {
                let server_channels: Vec<ChannelItem> = all_channels.iter()
                    .filter(|c| c.server_id == server.id)
                    .map(|c| ChannelItem {
                        id: c.id,
                        server_id: c.server_id,
                        name: c.name.clone(),
                        description: c.description.clone(),
                        position: c.position,
                    })
                    .collect();

                let server_members: Vec<MemberItem> = all_members.iter()
                    .filter(|(m, _)| m.server_id == server.id)
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

                let admins: Vec<Uuid> = all_members.iter()
                    .filter(|(m, _)| m.server_id == server.id && m.role == MemberRole::Admin)
                    .map(|(m, _)| m.user_id)
                    .collect();

                ServerItem {
                    id: server.id,
                    name: server.name,
                    description: server.description,
                    icon_url: server.icon_url,
                    owner_id: server.owner_id,
                    admins,
                    invitcode: server.invitcode,
                    channels: server_channels,
                    members: server_members 
                }
            })
        }).collect();

    Ok(GetServerResponse { server_list: servers })
}

pub async fn get_server_by_id(db: &DatabaseConnection, claims: Claims, id: i32) -> Result<GetServerIdResponse, AppError> {
    let memberships = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if memberships.is_none() {
        return Err(AppError::Forbidden("Not a member of this server".to_string()));
    }

    let server = server_model::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Server not found".to_string()))?;

    let channels = channel::Entity::find()
        .filter(channel::Column::ServerId.eq(id))
        .order_by_asc(channel::Column::Position)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .into_iter()
        .map(|c| ChannelItem {
            id: c.id,
            server_id: c.server_id,
            name: c.name,
            description: c.description,
            position: c.position,
        })
        .collect();

    let raw_members = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(id))
        .find_also_related(user::Entity)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    
    let admins: Vec<Uuid> = raw_members.iter()
        .filter(|(m, _)| m.role == MemberRole::Admin)
        .map(|(m, _)| m.user_id)
        .collect();

    let members: Vec<MemberItem> = raw_members
        .into_iter()
        .filter_map(|(m, u_opt)| {
            u_opt.map(|u| MemberItem {
                id: m.id,
                user_id: u.id,
                username: u.username,
                display_name: u.display_name,
                avatar_url: u.avatar_url,
                role: format!("{:?}", m.role),
                status: format!("{:?}", u.status),
            })
        })
        .collect();

    Ok(GetServerIdResponse {
        server: ServerItem {
            id: server.id,
            name: server.name,
            description: server.description,
            icon_url: server.icon_url,
            owner_id: server.owner_id,
            invitcode: server.invitcode,
            channels,
            admins,
            members
        }
    })
}

pub async fn update_server(db: &DatabaseConnection, claims: Claims, id: i32, req: UpdateServerRequest) -> Result<UpdateServerResponse, AppError> {
    // Vérifier que l'utilisateur est Owner ou Admin
    let membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member of this server".to_string()))?;

    if membership.role != MemberRole::Owner && membership.role != MemberRole::Admin {
        return Err(AppError::Forbidden("Only owners and admins can update server".to_string()));
    }

    // Récupérer le serveur
    let server = server_model::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Server not found".to_string()))?;

    // Créer l'ActiveModel pour la mise à jour
    let mut server_active: server_model::ActiveModel = server.into();

    let name_provided = req.name.is_some();
    let description_provided = req.description.is_some();
    let icon_url_provided = req.icon_url.is_some();

    // Mettre à jour uniquement les champs fournis
    if let Some(name) = req.name {
        if name.trim().is_empty() {
            return Err(AppError::BadRequest("Server name cannot be empty".to_string()));
        }
        if name.len() > 20 {
            return Err(AppError::BadRequest("Server name too long (max 20 characters)".to_string()));
        }
        server_active.name = Set(name.trim().to_string());
    }

    if let Some(description) = req.description {
        if description.trim().is_empty() {
            return Err(AppError::BadRequest("Description cannot be empty".to_string()));
        }
        server_active.description = Set(description.trim().to_string());
    }

    if let Some(icon_url) = req.icon_url {
        server_active.icon_url = Set(Some(icon_url));
    }

    // Sauvegarder les modifications
    let updated_server = server_active.update(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // Construire la réponse avec seulement les champs modifiés
    Ok(UpdateServerResponse {
        id: updated_server.id,
        new_name: if name_provided { Some(updated_server.name) } else { None },
        new_description: if description_provided { Some(updated_server.description) } else { None },
        new_icon_url: if icon_url_provided { updated_server.icon_url } else { None },
    })
}

pub async fn delete_server(db: &DatabaseConnection, claims: Claims, server_id: i32) -> Result<StatusCode, AppError> {
    let membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member of this server".to_string()))?;

    if membership.role != MemberRole::Owner {
        return Err(AppError::Forbidden("Only owners can delete server".to_string()))?;
    }

    let server = server_model::Entity::find_by_id(server_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Server not found".to_string()))?;

    let server_active: server_model::ActiveModel = server.into();
    let _ = server_active.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(StatusCode::OK)
}

pub async fn join_server(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims, 
    server_id: i32, 
    req: JoinServerRequest
) -> Result<JoinServerResponse, AppError> {
    // 1. Vérif Server
    let server = server_model::Entity::find_by_id(server_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Server not found".to_string()))?;

    let ban_record = server_ban::Entity::find()
        .filter(server_ban::Column::ServerId.eq(server_id))
        .filter(server_ban::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if let Some(ban) = ban_record {
        if let Some(banned_until) = ban.banned_until {
            if banned_until > chrono::Utc::now().naive_utc() {
                return Err(AppError::Forbidden("You are temporarily banned from this server".to_string()));
            } else {
                // Le ban a expiré, on le supprime de la base de données
                let ban_active: server_ban::ActiveModel = ban.into();
                let _ = ban_active.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;
            }
        } else {
            // Ban définitif (banned_until = None)
            return Err(AppError::Forbidden("You are permanently banned from this server".to_string()));
        }
    }

    // 2. Vérif déjà membre
    let existing_membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if existing_membership.is_some() {
        return Err(AppError::BadRequest("Already a member of this server".to_string()));
    }

    // 3. Vérif code
    if req.invitcode != server.invitcode {
        return Err(AppError::BadRequest("Invalid invitation code".to_string()));
    }

    let updated_membership_req = server_member::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(server_id),
        user_id: Set(claims.sub),
        role: Set(MemberRole::Member),
        ..Default::default()
    };
    let saved_member = updated_membership_req.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // --- BROADCAST ---
    let user_info = user::Entity::find_by_id(claims.sub)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    let ws_payload = json!({
        "type": "user_joined",
        "data": {
            "server_id": server_id,
            "member": {
                "id": saved_member.id,
                "user_id": user_info.id,
                "username": user_info.username.clone(),
                "display_name": user_info.display_name.clone(),
                "avatar_url": user_info.avatar_url.clone(),
                "role": "Member",
                "status": format!("{:?}", user_info.status)
            }
        }
    });
    let _ = tx.send(ws_payload.to_string());
    // -----------------

    let channels = channel::Entity::find()
        .filter(channel::Column::ServerId.eq(server_id))
        .order_by_asc(channel::Column::Position)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .into_iter()
        .map(|c| ChannelItem {
            id: c.id,
            server_id: c.server_id,
            name: c.name,
            description: c.description,
            position: c.position,
        })
        .collect();

    let raw_members = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .find_also_related(user::Entity)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let admins: Vec<Uuid> = raw_members.iter()
        .filter(|(m, _)| m.role == MemberRole::Admin)
        .map(|(m, _)| m.user_id)
        .collect();

    let members: Vec<MemberItem> = raw_members
        .into_iter()
        .filter_map(|(m, u_opt)| {
            u_opt.map(|u| MemberItem {
                id: m.id,
                user_id: u.id,
                username: u.username,
                display_name: u.display_name,
                avatar_url: u.avatar_url,
                role: format!("{:?}", m.role),
                status: format!("{:?}", u.status),
            })
        })
        .collect();

    Ok(JoinServerResponse {
        server: ServerItem {
            id: server.id,
            name: server.name,
            description: server.description,
            icon_url: server.icon_url,
            owner_id: server.owner_id,
            invitcode: server.invitcode,
            channels,
            admins,
            members 
        }
    })
}

pub async fn leave_server(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims, 
    server_id: i32
) -> Result<StatusCode, AppError> {
    // Vérif server
    let _server = server_model::Entity::find_by_id(server_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Server not found".to_string()))?;

    // Vérif membership
    let memberships = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member of this server".to_string()))?; // Convertit Option en Model ici

    if memberships.role == MemberRole::Owner {
        return Err(AppError::Forbidden("Owner cannot leave server. Delete the server instead".to_string()));
    }

    // Sauvegarde ID membre pour le broadcast avant suppression
    let member_id = memberships.id; 
    let user_id = memberships.user_id;

    let membership_active: server_member::ActiveModel = memberships.into();
    membership_active.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // --- BROADCAST USER_LEFT ---
    let ws_payload = json!({
        "type": "user_left",
        "data": {
            "server_id": server_id,
            "user_id": user_id,
            "member_id": member_id
        }
    });
    let _ = tx.send(ws_payload.to_string());
    // ---------------------------

    Ok(StatusCode::OK)
}

pub async fn get_servermembers(db: &DatabaseConnection, claims: Claims, server_id: i32) -> Result<GetServerMemberResponse, AppError> {
    let membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if membership.is_none() {
        return Err(AppError::Forbidden("Not a member of this server".to_string()));
    }

    let memberships = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .find_also_related(user::Entity)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let members: Vec<MemberItem> = memberships
        .into_iter()
        .filter_map(|(member, user_opt)| {
            user_opt.map(|user| MemberItem {
                id: member.id,
                user_id: user.id,
                username: user.username,
                display_name: user.display_name,
                avatar_url: user.avatar_url,
                role: format!("{:?}", member.role),
                status: format!("{:?}", user.status),
            })
        })
        .collect();

    Ok(GetServerMemberResponse { members })
}

pub async fn update_member(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims, 
    server_id: i32, 
    user_id: Uuid, 
    payload: UpdateMemberRequest
) -> Result<UpdateMemberResponse, AppError> {

    // 1. Vérifier le demandeur
    let requester_membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member of this server".to_string()))?;

    if requester_membership.role != MemberRole::Owner {
        return Err(AppError::Forbidden("Only the owner can update member roles".to_string()));
    }

    // 2. Vérifier la cible
    let target_membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::BadRequest("User is not a member of this server".to_string()))?;

    if claims.sub == user_id {
        return Err(AppError::Forbidden("Cannot modify your own role".to_string()));
    }

    let new_role = match payload.new_role.to_lowercase().as_str() {
        "admin" => MemberRole::Admin,
        "member" => MemberRole::Member,
        "owner" => MemberRole::Owner,
        _ => return Err(AppError::Forbidden("Invalid role. Use 'admin', 'member', or 'owner'".to_string())),
    };

    // 3. Gestion Spécifique du Transfert de Propriété (Owner)
    if new_role == MemberRole::Owner {
        let server = server_model::Entity::find_by_id(server_id)
            .one(db)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))?
            .ok_or(AppError::NotFound("Server not found".to_string()))?;

        // A. Mise à jour de la table Server
        let mut server_active: server_model::ActiveModel = server.into();
        server_active.owner_id = Set(user_id);
        server_active.update(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

        // B. Rétrogradation de l'ancien Owner en Admin
        let requester_user_id = requester_membership.user_id; // Sauvegarde ID
        let requester_membership_id = requester_membership.id; // Sauvegarde ID Membership

        let mut requester_active: server_member::ActiveModel = requester_membership.into();
        requester_active.role = Set(MemberRole::Admin);
        let _updated_requester = requester_active.update(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

        // --- BROADCAST 1 : Ancien Owner -> Admin ---
        let requester_user = user::Entity::find_by_id(requester_user_id)
            .one(db)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))?
            .unwrap(); // On sait qu'il existe

        let old_owner_item = MemberItem {
            id: requester_membership_id,
            user_id: requester_user.id,
            username: requester_user.username,
            display_name: requester_user.display_name,
            avatar_url: requester_user.avatar_url,
            role: "Admin".to_string(),
            status: format!("{:?}", requester_user.status), // ✅ Statut ajouté
        };

        let ws_payload_old_owner = json!({
            "type": "member_updated",
            "data": {
                "server_id": server_id,
                "member": old_owner_item
            }
        });
        let _ = tx.send(ws_payload_old_owner.to_string());
    }

    // 4. Mise à jour de la Cible (Le user passé en paramètres)
    let mut target_active: server_member::ActiveModel = target_membership.into();
    target_active.role = Set(new_role.clone());
    let updated_membership = target_active.update(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let user = user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    let member_item = MemberItem {
        id: updated_membership.id,
        user_id: user.id,
        username: user.username,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        role: match new_role {
            MemberRole::Admin => "Admin".to_string(),
            MemberRole::Member => "Member".to_string(),
            MemberRole::Owner => "Owner".to_string(),
        },
        status: format!("{:?}", user.status), // ✅ Statut ajouté
    };

    // --- BROADCAST 2 : Cible -> Nouveau Rôle ---
    let ws_payload = json!({
        "type": "member_updated",
        "data": {
            "server_id": server_id,
            "member": member_item.clone() 
        }
    });
    let _ = tx.send(ws_payload.to_string());

    Ok(UpdateMemberResponse {
        id: updated_membership.id,
        new_user: member_item,
    })
}

pub async fn create_channel(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>,
    claims: Claims, 
    server_id: i32, 
    req: CreateChannelRequest
) -> Result<ChannelItem, AppError> {

    // 1. Vérifier si l'utilisateur est Admin ou Owner du serveur
    let membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member of this server".to_string()))?;

    if membership.role != MemberRole::Owner && membership.role != MemberRole::Admin {
        return Err(AppError::Forbidden("Only Owner or Admin can create channels".to_string()));
    }

    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("Channel name cannot be empty".to_string()));
    }

    // 2. Création du Channel
    let new_channel = channel::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(server_id),
        name: Set(req.name),
        description: Set(req.description),
        position: Set(0), // Gestion de position à améliorer plus tard si besoin
        ..Default::default() 
    };

    let saved_channel = new_channel.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let channel_item = ChannelItem {
        id: saved_channel.id,
        server_id: saved_channel.server_id,
        name: saved_channel.name,
        description: saved_channel.description,
        position: saved_channel.position,
    };

    // --- 3. BROADCAST WEBSOCKET ---
    let ws_payload = json!({
        "type": "channel_created",
        "data": {
            "server_id": server_id,
            "channel": channel_item // On envoie l'objet complet
        }
    });

    // On diffuse. Le websocket.rs filtrera pour n'envoyer qu'aux membres de ce server_id
    let _ = tx.send(ws_payload.to_string());
    // ------------------------------

    Ok(channel_item)
}

pub async fn get_channels(db: &DatabaseConnection, claims: Claims, server_id: i32) -> Result<GetChannelsResponse, AppError>{
    let membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if membership.is_none() {
        return Err(AppError::Forbidden("Not a member of this server".to_string()));
    }

    let channels = channel::Entity::find()
        .filter(channel::Column::ServerId.eq(server_id))
        .order_by_asc(channel::Column::Position)
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let channel_items: Vec<ChannelItem> = channels
        .into_iter()
        .map(|c| ChannelItem {
            id: c.id,
            server_id: c.server_id,
            name: c.name,
            description: c.description, 
            position: c.position,
        })
        .collect();

    Ok(GetChannelsResponse { channels: channel_items })
}

pub async fn kick_user(
    db: &DatabaseConnection,
    tx: &broadcast::Sender<String>,
    claims: Claims,
    server_id: i32,
    user_id: Uuid
) -> Result<StatusCode, AppError> {
    // 1. Récupérer et vérifier le rôle du demandeur
    let requester_membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member of this server".to_string()))?;

    if requester_membership.role != MemberRole::Owner && requester_membership.role != MemberRole::Admin {
        return Err(AppError::Forbidden("Only Owner or Admin can kick users".to_string()));
    }

    // 2. Vérifier la cible du kick
    let target_membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::BadRequest("Target user is not a member of this server".to_string()))?;

    if target_membership.role == MemberRole::Owner {
        return Err(AppError::Forbidden("Cannot kick the server owner".to_string()));
    }

    if requester_membership.role == MemberRole::Admin && target_membership.role == MemberRole::Admin {
        return Err(AppError::Forbidden("Admins cannot kick other admins".to_string()));
    }

    if claims.sub == user_id {
        return Err(AppError::BadRequest("Cannot kick yourself. Use leave instead".to_string()));
    }

    let member_id = target_membership.id;

    // 3. Supprimer le membre
    let target_active: server_member::ActiveModel = target_membership.into();
    target_active.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // --- 4. BROADCAST ---
    let ws_payload = json!({
        "type": "user_kicked",
        "data": {
            "server_id": server_id,
            "user_id": user_id,
            "member_id": member_id
        }
    });
    let _ = tx.send(ws_payload.to_string());

    Ok(StatusCode::OK)
}

pub async fn ban_user(
    db: &DatabaseConnection,
    tx: &broadcast::Sender<String>,
    claims: Claims,
    server_id: i32,
    user_id: Uuid,
    req: BanUserRequest
) -> Result<StatusCode, AppError> {
    // 1. Récupérer et vérifier le rôle du demandeur
    let requester_membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member of this server".to_string()))?;

    if requester_membership.role != MemberRole::Owner && requester_membership.role != MemberRole::Admin {
        return Err(AppError::Forbidden("Only Owner or Admin can ban users".to_string()));
    }

    // 2. Vérifier la cible
    let target_membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(server_id))
        .filter(server_member::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if claims.sub == user_id {
        return Err(AppError::BadRequest("Cannot ban yourself".to_string()));
    }

    let member_id_str = if let Some(ref target) = target_membership {
        if target.role == MemberRole::Owner {
            return Err(AppError::Forbidden("Cannot ban the server owner".to_string()));
        }
        if requester_membership.role == MemberRole::Admin && target.role == MemberRole::Admin {
            return Err(AppError::Forbidden("Admins cannot ban other admins".to_string()));
        }
        Some(target.id.clone())
    } else {
        None
    };

    // 3. Kick l'utilisateur s'il est membre
    if let Some(target) = target_membership {
        let target_active: server_member::ActiveModel = target.into();
        target_active.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;
    }

    // 4. Ajouter l'utilisateur à la table des bans
    // duration est en secondes (ex: 10 = 10s, 3600 = 1h, 86400 = 1 jour)
    let banned_until = req.duration.map(|secs| chrono::Utc::now().naive_utc() + chrono::Duration::seconds(secs as i64));
    
    let new_ban = server_ban::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(server_id),
        user_id: Set(user_id),
        banned_by: Set(claims.sub),
        banned_until: Set(banned_until),
    };
    new_ban.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // --- 5. BROADCAST ---    
    let ws_payload = json!({
        "type": "user_banned",
        "data": {
            "server_id": server_id,
            "user_id": user_id,
            "member_id": member_id_str,
            "banned_until": banned_until.map(|d| d.to_string())
        }
    });
    
    let _ = tx.send(ws_payload.to_string());

    Ok(StatusCode::OK)
}