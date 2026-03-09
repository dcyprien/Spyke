use axum::{debug_handler,Json, http::StatusCode, response::IntoResponse, extract::{State, Path}};
use crate::{AppState, application::dto::server_dto::{CreateChannelRequest, JoinServerRequest, UpdateMemberRequest, UpdateServerRequest}};
use uuid::Uuid;
use crate::application::services::server_service;
use crate::application::dto::server_dto::{CreateServerRequest, BanUserRequest};
use crate::application::dto::token_dto::Claims;
use serde_json::json;


#[debug_handler]
pub async fn create_server(State(state): State<AppState>,claims: Claims, Json(payload): Json<CreateServerRequest>) -> impl IntoResponse {
    match server_service::create_server(&state.db, claims, payload).await {
        Ok(server) => (
            StatusCode::CREATED,
            Json(server)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn get_servers(State(state): State<AppState>,claims: Claims) -> impl IntoResponse {
    match server_service::get_servers(&state.db, claims).await {
        Ok(server) => (
            StatusCode::OK,
            Json(server)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn get_server_by_id(State(state): State<AppState>, claims: Claims, Path(server_id): Path<i32>) -> impl IntoResponse {
    match server_service::get_server_by_id(&state.db, claims, server_id).await {
        Ok(server) => (
            StatusCode::OK,
            Json(server)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn update_server(State(state): State<AppState>, claims:Claims, Path(server_id): Path<i32>, Json(payload): Json<UpdateServerRequest>) -> impl IntoResponse {
    match server_service::update_server(&state.db, claims, server_id, payload).await {
        Ok(server) => (
            StatusCode::OK,
            Json(server)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn delete_server(State(state): State<AppState>, claims:Claims, Path(server_id): Path<i32>) -> impl IntoResponse {
    match server_service::delete_server(&state.db, claims, server_id).await {
        Ok(_) => (
            StatusCode::OK
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn join_server(State(state): State<AppState>, claims: Claims, Path(server_id): Path<i32>, Json(payload): Json<JoinServerRequest>) -> impl IntoResponse {
    match server_service::join_server(&state.db, &state.tx, claims.clone(), server_id, payload).await {
        Ok(response) => {
            Json(response).into_response()
        }
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn leave_server(State(state): State<AppState>, claims: Claims, Path(server_id): Path<i32>) -> impl IntoResponse {
    match server_service::leave_server(&state.db, &state.tx, claims, server_id).await {
        Ok(_) => (
            StatusCode::OK
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn get_servermembers(State(state): State<AppState>, claims: Claims, Path(server_id): Path<i32>) -> impl IntoResponse {
    match server_service::get_servermembers(&state.db, claims, server_id).await {
        Ok(members) => (
            StatusCode::OK,
            Json(members)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn update_member(State(state): State<AppState>, claims:Claims, Path((server_id, user_id)): Path<(i32, Uuid)>, Json(payload): Json<UpdateMemberRequest>) -> impl IntoResponse {
 match server_service::update_member(&state.db, &state.tx, claims, server_id, user_id, payload).await {
        Ok(response) => (
            StatusCode::OK,
            Json(response)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn create_channel(State(state): State<AppState>, claims: Claims, Path(server_id): Path<i32>, Json(payload): Json<CreateChannelRequest>) -> impl IntoResponse {
    match server_service::create_channel(&state.db, &state.tx, claims, server_id, payload).await {
        Ok(_) => (
            StatusCode::OK,
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn get_channels(State(state): State<AppState>, claims: Claims, Path(server_id): Path<i32>) -> impl IntoResponse {
    match server_service::get_channels(&state.db, claims, server_id).await {
        Ok(response) => (
            StatusCode::OK,
            Json(response)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn kick_user(State(state): State<AppState>, claims: Claims, Path(server_id): Path<i32>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    match server_service::kick_user(&state.db, &state.tx, claims, server_id, user_id).await {
        Ok(_response) => (
            StatusCode::OK
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn ban_user(State(state): State<AppState>, claims: Claims, Path((server_id, user_id)): Path<(i32, Uuid)>, Json(payload): Json<BanUserRequest>) -> impl IntoResponse {
    match server_service::ban_user(&state.db, &state.tx, claims, server_id, user_id, payload).await {
        Ok(_response) => (
            StatusCode::OK
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}