use axum::{ Json, http::StatusCode, response::IntoResponse, extract::State };
use crate::AppState;
use crate::application::services::auth_service;
use crate::application::dto::auth_dto::{ LoginRequest, SignupRequest, UpdateStatusPayload};
use crate::application::dto::token_dto::Claims;
use crate::domain::models::user::UserStatus;
use serde::Deserialize;
use serde_json::json;

pub async fn signup(State(state): State<AppState>, Json(payload): Json<SignupRequest> ) -> impl IntoResponse {
    match auth_service::register_user(&state.db, payload).await {
        Ok(user) => (StatusCode::CREATED,
            Json(user)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    match auth_service::login_user(&state.db, &state.tx, payload).await {
        Ok(user) => (
            StatusCode::OK,
            Json(user)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn logout(State(state): State<AppState>, claims: Claims) -> impl IntoResponse {
    match auth_service::logout_user(&state.db, &state.tx, claims).await {
        Ok(response) => (
            StatusCode::OK,
            Json(response)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn me( State(state): State<AppState>, claims: Claims,) -> impl IntoResponse {
    match auth_service::me(&state.db, claims).await {
        Ok(user) => (
            StatusCode::OK,
            Json(user)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
    }
}

pub async fn update_status(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<UpdateStatusPayload>,
) -> impl IntoResponse {
    match auth_service::update_user_status(&state.db, &state.tx, claims.sub, payload.status).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{:?}", e)}))
        ).into_response(),
    }
}