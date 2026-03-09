use axum::{ Json, http::StatusCode, response::IntoResponse, extract::State };
use crate::AppState;
use crate::application::services::auth_service;
use crate::application::dto::auth_dto::{ LoginRequest, SignupRequest};
use crate::application::dto::token_dto::Claims;  // ✅ Import Claims
use serde_json::json;

pub async fn signup(State(state): State<AppState>, Json(payload): Json<SignupRequest> ) -> impl IntoResponse {
    match auth_service::register_user(&state.db, payload).await {
        Ok(user) => (StatusCode::CREATED,
            Json(user)
        ).into_response(),
        Err(e) => (e.status_code(), Json(json!({"error": e}))).into_response()
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
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": e}))
        ).into_response()
    }
}

pub async fn me( State(state): State<AppState>, claims: Claims,) -> impl IntoResponse {
    match auth_service::me(&state.db, claims).await {
        Ok(user) => (
            StatusCode::OK,
            Json(user)
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e}))
        ).into_response()
    }
}