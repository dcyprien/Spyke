use serde::{ Deserialize, Serialize };
use crate::application::dto::server_dto::ServerItem;
use crate::domain::models::user::UserStatus;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct BanInfo {
    pub server_id: i32,
    pub server_name: String,
    pub banned_until: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub password: String
}

#[derive(Debug, Serialize)]
pub struct SignupResponse{
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String
}

#[derive(Debug, Deserialize)]
pub struct MeRequest {
    pub id: Uuid,
    pub token: String
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusPayload {
    pub status: UserStatus,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub servers: Vec<ServerItem>,
    pub pending_bans: Vec<BanInfo>,
}