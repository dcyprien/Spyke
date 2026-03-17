use axum::extract::{State, Path, Json};
use axum::response::IntoResponse;
use axum::http::StatusCode;
use sea_orm::{DatabaseBackend, MockDatabase, DbErr};
use tokio::sync::broadcast;
use uuid::Uuid;
use std::sync::Arc;

// Remplacez 'backend' par le nom exact de votre crate
use backend::AppState;
use backend::application::dto::token_dto::Claims;
use backend::application::dto::message_dto::{SendMessageRequest, UpdateMessageRequest, ToggleReactionRequest};
use backend::infrastructure::api::handlers::message::{
    send_message, send_dm, get_messages, delete_message, 
    update_message, get_direct_messages, get_dm_list, toggle_reaction
};

// --- Nouveaux imports pour les mocks ---
use backend::domain::models::{channel, message, server_member, user, direct_message};
use backend::domain::models::user::UserStatus;
use chrono::Utc;

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

// Outil pour extraire le StatusCode d'une réponse Axum
async fn get_status(response: impl IntoResponse) -> StatusCode {
    response.into_response().status()
}

#[tokio::test]
async fn test_handler_send_message_error() {
    // Si la DB renvoie une erreur sur la première requête, le service va renvoyer une AppError
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB Error".to_string())])
        .into_connection();
    
    let state = State(create_app_state(db));
    let claims = create_claims(Uuid::new_v4());
    let path = Path(Uuid::new_v4());
    let payload = Json(SendMessageRequest { content: "Hello".to_string(), server_id: Some(1), target_id: None });

    let response = send_message(state, claims, path, payload).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_get_messages_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB Error".to_string())])
        .into_connection();
    
    let state = State(create_app_state(db));
    let claims = create_claims(Uuid::new_v4());
    let path = Path(Uuid::new_v4());

    let response = get_messages(state, claims, path).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_toggle_reaction_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB Error".to_string())])
        .into_connection();
    
    let state = State(create_app_state(db));
    let claims = create_claims(Uuid::new_v4());
    let path = Path(Uuid::new_v4());
    let payload = Json(ToggleReactionRequest { emoji: "👍".to_string() });

    let response = toggle_reaction(state, claims, path, payload).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_delete_message_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Find msg error".to_string())])
        .into_connection();
    
    let state = State(create_app_state(db));
    let claims = create_claims(Uuid::new_v4());
    let path = Path(Uuid::new_v4());

    let response = delete_message(state, claims, path).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

// --- SUITE : SEND MESSAGE (OK) ---

#[tokio::test]
async fn test_handler_send_message_ok() {
    let user_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: channel_id, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id, role: backend::domain::models::server_member::MemberRole::Member }]])
        .append_query_results(vec![vec![message::Model { id: Uuid::new_v4(), channel_id: Some(channel_id), server_id: Some(1), user_id, content: "H".to_string(), direct_message: None, created_at: Utc::now().into() }]])
        .append_query_results(vec![vec![user::Model { id: user_id, username: "U".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]])
        .into_connection();

    let state = State(create_app_state(db));
    let payload = Json(SendMessageRequest { content: "He".to_string(), server_id: Some(1), target_id: None });
    
    let response = send_message(state, create_claims(user_id), Path(channel_id), payload).await;
    assert_eq!(get_status(response).await, StatusCode::CREATED);
}

// --- SUITE : GET MESSAGES (OK) ---

#[tokio::test]
async fn test_handler_get_messages_ok() {
    let user_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Cherche le channel
        .append_query_results(vec![vec![channel::Model { 
            id: channel_id, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 
        }]])
        // 2. Regarde si l'utilisateur est membre du serveur
        .append_query_results(vec![vec![server_member::Model { 
            id: Uuid::new_v4(), server_id: 1, user_id, role: backend::domain::models::server_member::MemberRole::Member 
        }]])
        // 3. Renvoie la liste des messages (vide pour simplifier)
        .append_query_results(vec![vec![] as Vec<message::Model>])
        .into_connection();
        
    let state = State(create_app_state(db));
    let response = get_messages(state, create_claims(user_id), Path(channel_id)).await;
    assert_eq!(get_status(response).await, StatusCode::OK);
}

// --- SUITE : SEND DM (Error & OK) ---

#[tokio::test]
async fn test_handler_send_dm_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB Error".to_string())])
        .into_connection();
        
    let state = State(create_app_state(db));
    let payload = Json(SendMessageRequest { content: "Hello".to_string(), server_id: None, target_id: Some(Uuid::new_v4()) });
    
    let response = send_dm(state, create_claims(Uuid::new_v4()), Path(Uuid::new_v4()), payload).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_send_dm_ok() {
    let user_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![direct_message::Model { id: room_id, user1_id: user_id, user2_id: target_id, content: "".to_string(), created_at: Utc::now().into() }]])
        .append_query_results(vec![vec![message::Model { id: Uuid::new_v4(), channel_id: None, server_id: None, user_id, content: "H".to_string(), direct_message: Some(room_id), created_at: Utc::now().into() }]])
        .append_query_results(vec![vec![user::Model { id: user_id, username: "U".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]])
        .into_connection();

    let state = State(create_app_state(db));
    let payload = Json(SendMessageRequest { content: "H".to_string(), server_id: None, target_id: Some(target_id) });
    
    let response = send_dm(state, create_claims(user_id), Path(target_id), payload).await;
    assert_eq!(get_status(response).await, StatusCode::CREATED);
}

// --- SUITE : UPDATE MESSAGE (Error & OK) ---

#[tokio::test]
async fn test_handler_update_message_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB Error".to_string())])
        .into_connection();
        
    let state = State(create_app_state(db));
    let payload = Json(UpdateMessageRequest { new_content: "new".to_string() });
    
    let response = update_message(state, create_claims(Uuid::new_v4()), Path(Uuid::new_v4()), payload).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_update_message_ok() {
    let user_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![message::Model { id: msg_id, user_id, content: "old".to_string(), channel_id: None, server_id: None, direct_message: None, created_at: Utc::now().into() }]])
        .append_query_results(vec![vec![message::Model { id: msg_id, user_id, content: "new".to_string(), channel_id: None, server_id: None, direct_message: None, created_at: Utc::now().into() }]])
        .append_query_results(vec![vec![user::Model { id: user_id, username: "U".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]])
        .into_connection();
        
    let state = State(create_app_state(db));
    let payload = Json(UpdateMessageRequest { new_content: "new".to_string() });
    
    let response = update_message(state, create_claims(user_id), Path(msg_id), payload).await;
    assert_eq!(get_status(response).await, StatusCode::OK);
}

// --- SUITE : GET DIRECT MESSAGES (Error & OK) ---

#[tokio::test]
async fn test_handler_get_direct_messages_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB".to_string())])
        .into_connection();
        
    let state = State(create_app_state(db));
    let response = get_direct_messages(state, create_claims(Uuid::new_v4()), Path(Uuid::new_v4())).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_get_direct_messages_ok() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<direct_message::Model>]) 
        .into_connection();
        
    let state = State(create_app_state(db));
    let response = get_direct_messages(state, create_claims(Uuid::new_v4()), Path(Uuid::new_v4())).await;
    assert_eq!(get_status(response).await, StatusCode::OK);
}

// --- SUITE : GET DM LIST (Error & OK) ---

#[tokio::test]
async fn test_handler_get_dm_list_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB".to_string())])
        .into_connection();
        
    let state = State(create_app_state(db));
    let response = get_dm_list(state, create_claims(Uuid::new_v4())).await;
    assert_eq!(get_status(response).await, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_get_dm_list_ok() {
    let uid1 = Uuid::new_v4();
    let uid2 = Uuid::new_v4();
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Cherche les DM
        .append_query_results(vec![vec![direct_message::Model { 
            id: Uuid::new_v4(), user1_id: uid1, user2_id: uid2, content: "".to_string(), created_at: Utc::now().into() 
        }]])
        // 2. Cherche les User associes
        .append_query_results(vec![vec![
            user::Model { id: uid1, username: "U1".to_string(), display_name: None, avatar_url: None, password_hash: "x".to_string(), status: UserStatus::Online },
            user::Model { id: uid2, username: "U2".to_string(), display_name: None, avatar_url: None, password_hash: "x".to_string(), status: UserStatus::Online }
        ]])
        .into_connection();
        
    let state = State(create_app_state(db));
    let response = get_dm_list(state, create_claims(uid1)).await;
    assert_eq!(get_status(response).await, StatusCode::OK);
}