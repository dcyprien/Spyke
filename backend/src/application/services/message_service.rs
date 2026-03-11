use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::{ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, ActiveModelTrait, QueryOrder};
use uuid::Uuid;
use tokio::sync::broadcast;
use serde_json::json;
use crate::{application::dto::{apperror::AppError, message_dto::{GetMessagesResponse, MessageItem, SendMessageRequest, UpdateMessageRequest, UpdateMessageResponse}, token_dto::Claims}, domain::models::{channel, message, server_member::{self, MemberRole}, user, direct_message}};

// 1. ADAPTATION DE SEND_MESSAGE
pub async fn send_message(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims, 
    channel_id: Uuid, 
    req: SendMessageRequest
) -> Result<MessageItem, AppError> { // Retourne MessageItem complet

    let req_server_id = req.server_id.ok_or(AppError::BadRequest("server_id is required for channel messages".to_string()))?;

    // 1. Vérifications existantes
    let channel = channel::Entity::find_by_id(channel_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    if channel.server_id != req_server_id { return Err(AppError::BadRequest("Channel error".to_string())); }

    let _membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(req.server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member".to_string()))?;

    if req.content.trim().is_empty() { return Err(AppError::BadRequest("Empty content".to_string())); }

    // 2. Insertion DB
    let new_message = message::ActiveModel {
        id: Set(Uuid::new_v4()),
        channel_id: Set(Some(channel_id)),
        server_id: Set(Some(req_server_id)),
        user_id: Set(claims.sub), 
        content: Set(req.content.clone()),
        direct_message: Set(req.direct_message),
        created_at: Set(Utc::now()), 
        ..Default::default()
    };

    let saved_msg = new_message.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // 3. Récupération infos User (on en a besoin pour le broadcast ET le return)
    let user_info = user::Entity::find_by_id(claims.sub)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::InternalServerError("User not found".to_string()))?;

    // 4. BROADCAST VIA WEBSOCKET
    // On structure le message pour qu'il soit compatible avec la logique de filtrage dans websocket.rs
    let ws_payload = json!({
        "type": "new_message",
        "data": {
            "id": saved_msg.id,
            "content": saved_msg.content,
            "user_id": saved_msg.user_id,
            "author_username": user_info.username, // Pseudo pour le WS
            "server_id": saved_msg.server_id,
            "channel_id": saved_msg.channel_id,
            "created_at": saved_msg.created_at.to_string()
        }
    });
    let _ = tx.send(ws_payload.to_string());

    // Retour HTTP avec le champ author rempli
    Ok(MessageItem {
        id: saved_msg.id,
        content: saved_msg.content,
        user_id: saved_msg.user_id,
        author: user_info.username, // ✅ Ajout ici
        server_id: saved_msg.server_id,
        channel_id: saved_msg.channel_id,
        direct_message_id: saved_msg.direct_message, // ✅ Ajout du DM
        created_at: saved_msg.created_at.into(),
    })
}

pub async fn send_dm(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims, 
    dm_id: Uuid,
    req: SendMessageRequest
) -> Result<MessageItem, AppError> {

    if req.content.trim().is_empty() { return Err(AppError::BadRequest("Empty content".to_string())); }

    // 1. Insertion DB (Sans server ni channel, mais avec direct_message)
    let new_message = message::ActiveModel {
        id: Set(Uuid::new_v4()),
        channel_id: Set(None),
        server_id: Set(None),
        user_id: Set(claims.sub), 
        content: Set(req.content.clone()),
        direct_message: Set(Some(dm_id)),
        created_at: Set(Utc::now()), 
        ..Default::default()
    };

    let saved_msg = new_message.insert(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // 2. Récupération infos User
    let user_info = user::Entity::find_by_id(claims.sub)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::InternalServerError("User not found".to_string()))?;

    // 3. BROADCAST VIA WEBSOCKET
    let ws_payload = json!({
        "type": "new_dm_message",
        "data": {
            "id": saved_msg.id.to_string(),
            "content": saved_msg.content,
            "user_id": saved_msg.user_id.to_string(),
            "author_username": user_info.username.clone(),
            "direct_message_id": dm_id.to_string(),
            "created_at": saved_msg.created_at.to_string()
        }
    });
    let _ = tx.send(ws_payload.to_string());

    // 4. Retour HTTP
    Ok(MessageItem {
        id: saved_msg.id,
        content: saved_msg.content,
        user_id: saved_msg.user_id,
        author: user_info.username,
        server_id: None,
        channel_id: None,
        direct_message_id: Some(dm_id),
        created_at: saved_msg.created_at.into(),
    })
}

// 2. ADAPTATION DE GET_MESSAGES
pub async fn get_messages(db: &DatabaseConnection, claims: Claims, channel_id: Uuid) -> Result<GetMessagesResponse, AppError> {
    
    // 1. Vérifications existantes
    let channel_model = channel::Entity::find_by_id(channel_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    let _membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(channel_model.server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("Not a member".to_string()))?;

    // REQUÊTE MODIFIÉE : On fait une Jointure (find_also_related) avec User
    let messages = message::Entity::find()
        .filter(message::Column::ChannelId.eq(channel_id))
        .order_by_asc(message::Column::CreatedAt)
        .find_also_related(user::Entity) // ✅ JOIN USER
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let message_list = messages.into_iter().map(|(msg, user_opt)| {
        // user_opt est Option<user::Model> car il vient du LEFT JOIN
        let author_name = match user_opt {
            Some(u) => u.username,
            None => "Utilisateur Inconnu".to_string() // Fallback si user supprimé
        };

        MessageItem {
            id: msg.id,
            content: msg.content,
            user_id: msg.user_id,
            author: author_name,
            server_id: msg.server_id,
            channel_id: msg.channel_id,
            direct_message_id: msg.direct_message,
            created_at: msg.created_at.into(), 
        }
    }).collect();

    Ok(GetMessagesResponse { message_list })
}

// AJOUTEZ l'argument tx: &broadcast::Sender<String>
pub async fn delete_message(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims, 
    message_id: Uuid
) -> Result<StatusCode, AppError> {
    
    // 1. Récupérer le message
    let message_model = message::Entity::find_by_id(message_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Message not found".to_string()))?;

    // On stocke les IDs pour le broadcast avant de supprimer
    let server_id = message_model.server_id;
    let channel_id = message_model.channel_id;
    let direct_message_id = message_model.direct_message;

    // Logique de permission (inchangée)
    // Cas simple : L'utilisateur est l'auteur
    let authorized = if message_model.user_id == claims.sub {
        true
    } else if let Some(c_id) = channel_id {
        // Cas admin/owner    
        let channel_model = channel::Entity::find_by_id(c_id)
            .one(db)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))?
            .ok_or(AppError::NotFound("Channel not found".to_string()))?;

        let membership = server_member::Entity::find()
            .filter(server_member::Column::ServerId.eq(channel_model.server_id))
            .filter(server_member::Column::UserId.eq(claims.sub))
            .one(db)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))?
            .ok_or(AppError::Forbidden("Not a member".to_string()))?;

        membership.role == MemberRole::Admin || membership.role == MemberRole::Owner
    } else {
        false
    };

    if authorized {
        // 2. SUPPRESSION EN DB
        message_model.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

        // 3. BROADCAST VIA WEBSOCKET (C'est ici qu'on garanti l'envoi)
        let event = json!({
            "type": "DELETE_MESSAGE",
            "data": {
                // On convertit en String pour éviter les conflits de type Int/String dans websocket.rs
                "server_id": server_id.map(|id| id.to_string()), 
                "channel_id": channel_id.map(|id| id.to_string()),
                "direct_message": direct_message_id.map(|id| id.to_string()),
                "message_id": message_id.to_string()
            }
        });
        
        // On envoie dans le canal
        let _ = tx.send(event.to_string());
        
        return Ok(StatusCode::OK);
    } else {
        Err(AppError::Forbidden("Permission denied".to_string()))
    }
}

pub async fn update_message(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, 
    claims: Claims, 
    message_id: Uuid, 
    req: UpdateMessageRequest
) -> Result<UpdateMessageResponse, AppError> {
    
    // 1. Récupérer le message original
    let message_model = message::Entity::find_by_id(message_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Message not found".to_string()))?;

    // 2. Vérification Permission : Uniquement l'auteur
    if message_model.user_id != claims.sub {
        return Err(AppError::Forbidden("You can only edit your own messages".to_string()));
    }

    // 3. Validation du contenu (pas vide)
    if req.new_content.trim().is_empty() {
        return Err(AppError::BadRequest("Message content cannot be empty".to_string()));
    }

    // 4. Mise à jour en DB
    let mut active_msg: message::ActiveModel = message_model.into();
    active_msg.content = Set(req.new_content.clone());

    let updated_msg = active_msg.update(db).await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // 5. Récupération de l'auteur (pour renvoyer l'objet complet MessageItem)
    let author_user = user::Entity::find_by_id(updated_msg.user_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::InternalServerError("Author user not found".to_string()))?;

    // 6. Broadcast via WebSocket
    // On envoie un event "UPDATE_MESSAGE" avec le nouveau contenu
    let event = json!({
        "type": "UPDATE_MESSAGE",
        "data": {
            // Conversion en String importante pour le routage WS
            "server_id": updated_msg.server_id.map(|id| id.to_string()),
            "channel_id": updated_msg.channel_id.map(|id| id.to_string()),
            "direct_message": updated_msg.direct_message.map(|id| id.to_string()),
            "message_id": updated_msg.id.to_string(),
            "new_content": updated_msg.content
        }
    });

    // Envoi sans bloquer si erreur (pas de receiver)
    let _ = tx.send(event.to_string());

    // 7. Retour de la réponse API
    Ok(UpdateMessageResponse {
        new_message: MessageItem {
            id: updated_msg.id,
            content: updated_msg.content,
            user_id: updated_msg.user_id,
            author: author_user.username,
            channel_id: updated_msg.channel_id,
            server_id: updated_msg.server_id,
            direct_message_id: updated_msg.direct_message,
            created_at: updated_msg.created_at.into()
        }
    })
}

pub async fn get_direct_messages(db: &DatabaseConnection, claims: Claims, dm_id: Uuid) -> Result<GetMessagesResponse, AppError> {
    
    // 1. Vérification que la room ("direct_message") existe et que l'utilisateur y a accès
    let dm_room = crate::domain::models::direct_message::Entity::find_by_id(dm_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Direct message conversation not found".to_string()))?;

    // Seulement le sender ou le receiver peuvent lire cette conversation
     if dm_room.user1_id != claims.sub && dm_room.user2_id != claims.sub {
        return Err(AppError::Forbidden("You are not part of this conversation".to_string()));
    }

    // 2. Récupérer les messages liés à cet ID de DM
    let messages = message::Entity::find()
        .filter(message::Column::DirectMessage.eq(Some(dm_id)))
        .order_by_asc(message::Column::CreatedAt)
        .find_also_related(user::Entity) // ✅ JOIN USER pour le nom
        .all(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // 3. Formater la réponse
    let message_list = messages.into_iter().map(|(msg, user_opt)| {
        let author_name = match user_opt {
            Some(u) => u.username,
            None => "Utilisateur Inconnu".to_string()
        };

        MessageItem {
            id: msg.id,
            content: msg.content,
            user_id: msg.user_id,
            author: author_name,
            server_id: msg.server_id,
            channel_id: msg.channel_id,
            direct_message_id: msg.direct_message,
            created_at: msg.created_at.into(), 
        }
    }).collect();

    Ok(GetMessagesResponse { message_list })
}