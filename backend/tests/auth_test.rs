use axum::extract::{State, Json};
use axum::response::IntoResponse;
use axum::http::StatusCode;
use sea_orm::{DatabaseBackend, MockDatabase, DbErr, MockExecResult};
use tokio::sync::broadcast;
use uuid::Uuid;
use std::sync::Arc;
use chrono::Utc;

// Ajustez "backend" par le nom exact de votre crate (probablement "backend" vu les fichiers)
use backend::AppState;
use backend::application::dto::token_dto::Claims;
use backend::application::dto::auth_dto::{SignupRequest, LoginRequest, UpdateStatusPayload};
use backend::infrastructure::api::handlers::auth::{signup, login, logout, me, update_status};
use backend::domain::models::{user, refresh_token, server_member, server_model, channel, server_ban};
use backend::domain::models::user::UserStatus;

use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHasher};

fn create_claims(user_id: Uuid) -> Claims {
    Claims { sub: user_id, exp: 10000000000, iat: 10000000000, username: "testuser".to_string() }
}

fn create_app_state(db: sea_orm::DatabaseConnection) -> AppState {
    let (tx, _) = broadcast::channel(100);
    AppState {
        db: Arc::new(db),
        tx,
    }
}

async fn get_status(response: impl IntoResponse) -> StatusCode {
    response.into_response().status()
}

fn generate_hash(pwd: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(pwd.as_bytes(), &salt).unwrap().to_string()
}

// --- SUITE : SIGNUP ---

#[tokio::test]
async fn test_handler_signup_err_short_password() {
    let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
    let state = State(create_app_state(db));
    let payload = Json(SignupRequest { username: "u".to_string(), password: "123".to_string() }); // Trop court
    
    let res = signup(state, payload).await;
    assert_eq!(get_status(res).await, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_handler_signup_ok() {
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<user::Model>]) // Pas d'user existant
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Offline, display_name: None, avatar_url: None }]]) // Insert
        .into_connection();

    let state = State(create_app_state(db));
    let payload = Json(SignupRequest { username: "u".to_string(), password: "password123".to_string() });
    
    let res = signup(state, payload).await;
    assert_eq!(get_status(res).await, StatusCode::CREATED);
}

// --- SUITE : LOGIN ---

#[tokio::test]
async fn test_handler_login_err_not_found() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<user::Model>]) // NotFound
        .into_connection();

    let state = State(create_app_state(db));
    let payload = Json(LoginRequest { username: "u".to_string(), password: "pwd".to_string() });
    
    let res = login(state, payload).await;
    assert_eq!(get_status(res).await, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_handler_login_ok() {
    std::env::set_var("JWT_SECRET", "supersecret");
    let user_id = Uuid::new_v4();
    let hash = generate_hash("password123");

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find user
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: hash, status: UserStatus::Offline, display_name: None, avatar_url: None }]])
        // 2. Update status -> Online
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]])
        // 3. Find memberships
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // 4. Find existing token
        .append_query_results(vec![vec![] as Vec<refresh_token::Model>])
        // 5. Insert new token
        .append_query_results(vec![vec![refresh_token::Model { id: Uuid::new_v4(), user_id, token: "tok".to_string(), expires_at: Utc::now().into(), created_at: Utc::now().into() }]])
        .into_connection();

    let state = State(create_app_state(db));
    let payload = Json(LoginRequest { username: "u".to_string(), password: "password123".to_string() });
    
    let res = login(state, payload).await;
    assert_eq!(get_status(res).await, StatusCode::OK);
}

// --- SUITE : LOGOUT ---

#[tokio::test]
async fn test_handler_logout_db_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Membership Fail".to_string())])
        .into_connection();

    let state = State(create_app_state(db));
    let res = logout(state, create_claims(Uuid::new_v4())).await;
    assert_eq!(get_status(res).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_logout_ok() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // Update offline returns the model
        .append_query_results(vec![vec![user::Model { id: Uuid::new_v4(), username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Offline, display_name: None, avatar_url: None }]])
        // Memberships find
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // Delete refresh token -> returns ExecResult
        .append_exec_results(vec![MockExecResult { last_insert_id: 0, rows_affected: 1 }])
        .into_connection();

    let state = State(create_app_state(db));
    let res = logout(state, create_claims(Uuid::new_v4())).await;
    assert_eq!(get_status(res).await, StatusCode::OK);
}

// --- SUITE : ME ---

#[tokio::test]
async fn test_handler_me_err() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("User fetching crash".to_string())])
        .into_connection();

    let state = State(create_app_state(db));
    let res = me(state, create_claims(Uuid::new_v4())).await;
    assert_eq!(get_status(res).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_me_ok() {
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]])
        .append_query_results(vec![vec![] as Vec<(server_member::Model, Option<server_model::Model>)>]) // memberships with servers
        .append_query_results(vec![vec![] as Vec<channel::Model>]) // channels
        .append_query_results(vec![vec![] as Vec<(server_member::Model, Option<user::Model>)>]) // members
        .append_query_results(vec![vec![] as Vec<(server_ban::Model, Option<server_model::Model>)>]) // bans
        .into_connection();

    let state = State(create_app_state(db));
    let res = me(state, create_claims(user_id)).await;
    assert_eq!(get_status(res).await, StatusCode::OK);
}

// --- SUITE : UPDATE STATUS ---

#[tokio::test]
async fn test_handler_update_status_err() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB".to_string())])
        .into_connection();

    let state = State(create_app_state(db));
    let payload = Json(UpdateStatusPayload { status: UserStatus::Invisible });

    let res = update_status(state, create_claims(Uuid::new_v4()), payload).await;
    assert_eq!(get_status(res).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_update_status_ok() {
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]])
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Offline, display_name: None, avatar_url: None }]])
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .into_connection();

    let state = State(create_app_state(db));
    let payload = Json(UpdateStatusPayload { status: UserStatus::Offline });

    let res = update_status(state, create_claims(user_id), payload).await;
    assert_eq!(get_status(res).await, StatusCode::OK);
}