use axum::{ Json, http::StatusCode, response::IntoResponse, extract::{State, Multipart} };
use crate::AppState;
use crate::application::services::auth_service;
use crate::application::dto::auth_dto::{ LoginRequest, SignupRequest, UpdateStatusPayload};
use crate::application::dto::token_dto::Claims;
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

pub async fn upload_avatar(
    State(state): State<AppState>,
    claims: Claims,
    mut multipart: Multipart,
) -> impl IntoResponse {
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(field_name) = field.name() {
            if field_name == "avatar" {
                match field.bytes().await {
                    Ok(image_data) => {
                        match auth_service::update_avatar(&state.db, &state.tx, claims.sub, image_data.to_vec()).await {
                            Ok(avatar_url) => {
                                return (
                                    StatusCode::OK,
                                    Json(json!({"avatar_url": avatar_url}))
                                ).into_response();
                            },
                            Err(e) => {
                                return (
                                    e.status_code(),
                                    Json(json!({"error": format!("{:?}", e)}))
                                ).into_response();
                            }
                        }
                    },
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": "Failed to read file"}))
                        ).into_response();
                    }
                }
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "No avatar field provided"}))
    ).into_response()
}