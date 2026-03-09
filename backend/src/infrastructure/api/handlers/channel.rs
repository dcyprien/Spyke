use crate::{AppState, application::{dto::{channel_dto::{TypingEvent, UpdateChannelRequest}, token_dto::Claims}, services::channel_service}};
use serde_json::json;
use axum::{Json, http::StatusCode, response::IntoResponse, extract::{State, Path}};
use uuid::Uuid;

pub async fn get_channel_by_id(State(state): State<AppState>, claims: Claims, Path(channel_id): Path<Uuid>) -> impl IntoResponse {
    match channel_service::get_channel_by_id(&state.db, claims, channel_id).await {
        Ok(response) => (
            StatusCode::OK,
            Json(response)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn update_channel(State(state): State<AppState>, claims: Claims, Path(channel_id): Path<Uuid>, Json(payload): Json<UpdateChannelRequest>) -> impl IntoResponse {
    // PASSAGE DU BROADCAST (state.tx)
    match channel_service::update_channel(&state.db, &state.tx, claims, channel_id, payload).await {
        Ok(response) => (
            StatusCode::OK,
            Json(response)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn delete_channel(State(state): State<AppState>, claims: Claims, Path(channel_id): Path<Uuid>) -> impl IntoResponse {
    match channel_service::delete_channel(&state.db, claims, channel_id).await {
        Ok(_) => (
            StatusCode::OK
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn send_typing_status(State(state): State<AppState>, claims: Claims, Path(channel_id): Path<Uuid>, Json(payload): Json<TypingEvent>) -> impl IntoResponse {
    let event = json!({
        "type": "IS_TYPING",
        "data": {
            "server_id": payload.server_id,
            "channel_id": channel_id,
            "user_id": claims.sub,
            "is_typing": true
        }
    });

    let _ = state.tx.send(event.to_string());
    StatusCode::OK
}