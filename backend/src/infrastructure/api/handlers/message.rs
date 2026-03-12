use axum::{Json, http::StatusCode, response::IntoResponse, extract::{State, Path}};
use crate::{AppState, application::{dto::message_dto::{SendMessageRequest, UpdateMessageRequest}, services::message_service}};
use uuid::Uuid;
use sea_orm::{EntityTrait}; 
use crate::application::dto::token_dto::Claims;
use crate::domain::models::message;
use serde_json::json;

pub async fn send_message(State(state): State<AppState>, claims: Claims, Path(channel_id): Path<Uuid>, Json(payload): Json<SendMessageRequest>) -> impl IntoResponse {
    match message_service::send_message(&state.db, &state.tx, claims.clone(), channel_id, payload.clone()).await {
        Ok(msg) => (StatusCode::CREATED, Json(msg)).into_response(), // Retourner le JSON permet au front de l'avoir directement
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn send_dm(State(state): State<AppState>, claims: Claims, Path(target_user_id): Path<Uuid>, Json(payload): Json<SendMessageRequest>) -> impl IntoResponse {
    match message_service::send_dm(&state.db, &state.tx, claims.clone(), target_user_id, payload.clone()).await {
        Ok(msg) => (StatusCode::CREATED, Json(msg)).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn get_messages(State(state): State<AppState>, claims: Claims, Path(channel_id): Path<Uuid>) -> impl IntoResponse {
    match message_service::get_messages(&state.db, claims, channel_id).await {
        Ok(list) => (
            StatusCode::OK,
            Json(list)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn delete_message(State(state): State<AppState>, claims: Claims, Path(message_id): Path<Uuid>) -> impl IntoResponse {
    let _message_opt = message::Entity::find_by_id(message_id)
        .one(&*state.db) 
        .await
        .ok()
        .flatten();
        
    match message_service::delete_message(&state.db,  &state.tx, claims, message_id).await {
        Ok(_) => {
            StatusCode::OK.into_response()
        }
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn update_message(State(state): State<AppState>, claims: Claims, Path(message_id): Path<Uuid>, Json(payload): Json<UpdateMessageRequest>) -> impl IntoResponse {
    match message_service::update_message(&state.db, &state.tx, claims, message_id, payload).await {
        Ok(response) => (
            StatusCode::OK,
            Json(response)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn get_direct_messages(State(state): State<AppState>, claims: Claims, Path(dm_id): Path<Uuid>) -> impl IntoResponse {
    match message_service::get_direct_messages(&state.db, claims, dm_id).await {
        Ok(msgs) => (StatusCode::OK, Json(msgs)).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn get_dm_list(State(state): State<AppState>, claims: Claims) -> impl IntoResponse {
    match message_service::get_dm_list(&state.db, claims).await {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}