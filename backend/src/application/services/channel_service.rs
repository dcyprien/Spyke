use axum::http::StatusCode;
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set, IntoActiveModel};
use uuid::Uuid;
use crate::application::dto::apperror::AppError;
use crate::domain::models::server_member::MemberRole;
use tokio::sync::broadcast; // Import broadcast
use serde_json::json; // Import json marco

use crate::application::dto::{channel_dto::{ChannelItem, GetChannelResponse, UpdateChannelRequest, UpdateChannelResponse}, token_dto::Claims};
use crate::domain::models::{channel, server_member};

pub async fn get_channel_by_id(db : &DatabaseConnection, claims:Claims, channel_id: Uuid) -> Result<GetChannelResponse, AppError> {
    let channel_model = channel::Entity::find_by_id(channel_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    let is_member = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(channel_model.server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .is_some();

    if !is_member {
        return Err(AppError::Forbidden("Access denied: You are not a member of the server containing this channel".to_string()));
    }

    Ok(GetChannelResponse {
        channel: ChannelItem {
            id: channel_model.id,
            server_id: channel_model.server_id,
            name: channel_model.name,
            description: channel_model.description,
            position: channel_model.position,
        }
    })
}

pub async fn update_channel(
    db: &DatabaseConnection, 
    tx: &broadcast::Sender<String>, // AJOUT DU CANAL DE BROADCAST
    claims: Claims, 
    channel_id: Uuid, 
    req: UpdateChannelRequest
) -> Result<UpdateChannelResponse, AppError> {
    
    let channel_model = channel::Entity::find_by_id(channel_id)
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
        .ok_or(AppError::Forbidden("Not a member of the server".to_string()))?;

    if membership.role != MemberRole::Owner && membership.role != MemberRole::Admin {
        return Err(AppError::Forbidden("Insufficient permissions: Only Owner or Admin can update channels".to_string()));
    }

    let mut channel_active: channel::ActiveModel = channel_model.into_active_model();

    if let Some(name) = req.name {
        if name.trim().is_empty() {
             return Err(AppError::BadRequest("Channel name cannot be empty".to_string()));
        }
        channel_active.name = Set(name);
    }

    // Rétablissement de la logique pour la description avec validation
    if let Some(desc) = req.description {
        if desc.trim().is_empty() {
             return Err(AppError::BadRequest("Channel description cannot be empty".to_string()));
        }
        channel_active.description = Set(desc);
    }

    if let Some(pos) = req.position {
        channel_active.position = Set(pos);
    }

    let updated_channel = channel_active.update(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // --- BROADCAST DU MESSAGE UPDATE_CHANNEL ---
    let event = json!({
        "type": "UPDATE_CHANNEL",
        "data": {
            "server_id": updated_channel.server_id, // Integer brut
            "channel_id": updated_channel.id.to_string(),
            "name": updated_channel.name,
            "description": updated_channel.description
        }
    });

    // Envoi dans le tunnel
    let _ = tx.send(event.to_string());
    // -------------------------------------------

    Ok(UpdateChannelResponse {
        channel: ChannelItem {
            id: updated_channel.id,
            server_id: updated_channel.server_id,
            name: updated_channel.name,
            description: updated_channel.description,
            position: updated_channel.position,
        }
    })
}

pub async fn delete_channel(db: &DatabaseConnection, claims: Claims, channel_id: Uuid) -> Result<StatusCode, AppError> {
    let channel = channel::Entity::find_by_id(channel_id)
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    let membership = server_member::Entity::find()
        .filter(server_member::Column::ServerId.eq(channel.server_id))
        .filter(server_member::Column::UserId.eq(claims.sub))
        .one(db)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or(AppError::Forbidden("You are not a member of this server".to_string()))?;

    if membership.role != MemberRole::Admin && membership.role != MemberRole::Owner {
        return Err(AppError::Forbidden("Only owners and admins can delete a channel".to_string()));
    }
     let channel_active = channel.into_active_model();

     let _ = channel_active.delete(db).await.map_err(|e| AppError::InternalServerError(e.to_string()))?;
     Ok(StatusCode::OK)
}
