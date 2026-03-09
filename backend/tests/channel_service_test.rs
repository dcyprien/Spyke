use sea_orm::{
    DatabaseBackend, MockDatabase, MockExecResult, DbErr, 
    EntityTrait, ActiveModelTrait
};
use tokio::sync::broadcast;
use uuid::Uuid;
use axum::http::StatusCode;

// Imports internes
use backend::application::services::channel_service;
use backend::application::dto::channel_dto::UpdateChannelRequest;
use backend::application::dto::token_dto::Claims;
use backend::application::dto::apperror::AppError;
use backend::domain::models::{channel, server_member, server_member::MemberRole};

// Helper
fn create_claims(user_id: Uuid) -> Claims {
    Claims {
        sub: user_id,
        username: "test_tester".to_string(),
        exp: 10000000000,
        iat: 10000000000,
    }
}

// --- SUITE 1 : GET CHANNEL ---

#[tokio::test]
async fn test_get_channel_success() {
    let chan_id = Uuid::new_v4();
    let srv_id = 1;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Channel
        .append_query_results(vec![vec![channel::Model {
            id: chan_id, server_id: srv_id, name: "General".to_string(), description: "Desc".to_string(), position: 0
        }]])
        // 2. Check Membership
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        .into_connection();

    let res = channel_service::get_channel_by_id(&db, create_claims(user_id), chan_id).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().channel.name, "General");
}

#[tokio::test]
async fn test_get_channel_not_found() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<channel::Model>])
        .into_connection();

    let res = channel_service::get_channel_by_id(&db, create_claims(Uuid::new_v4()), Uuid::new_v4()).await;
    assert!(matches!(res, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_get_channel_forbidden() {
    let chan_id = Uuid::new_v4();
    let srv_id = 10;
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model {
            id: chan_id, server_id: srv_id, name: "G".to_string(), description: "".to_string(), position: 0
        }]])
        // Membre non trouvé (Vec vide)
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .into_connection();

    let res = channel_service::get_channel_by_id(&db, create_claims(Uuid::new_v4()), chan_id).await;
    assert!(matches!(res, Err(AppError::Forbidden(msg)) if msg.contains("Access denied")));
}

// Tests DB Errors (Get)
#[tokio::test]
async fn test_get_channel_db_errors() {
    let uid = Uuid::new_v4();
    let cid = Uuid::new_v4();

    // 1. Fetch Channel Fail
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Fetch Channel Error".to_string())])
        .into_connection();
    assert!(matches!(channel_service::get_channel_by_id(&db1, create_claims(uid), cid).await, Err(AppError::InternalServerError(_))));

    // 2. Fetch Membership Fail
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "G".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_errors(vec![DbErr::Custom("Fetch Member Error".to_string())])
        .into_connection();
    assert!(matches!(channel_service::get_channel_by_id(&db2, create_claims(uid), cid).await, Err(AppError::InternalServerError(_))));
}

// --- SUITE 2 : UPDATE CHANNEL ---

#[tokio::test]
async fn test_update_channel_success() {
    let chan_id = Uuid::new_v4();
    let srv_id = 55;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Channel
        .append_query_results(vec![vec![channel::Model {
            id: chan_id, server_id: srv_id, name: "Old".to_string(), description: "D".to_string(), position: 0
        }]])
        // 2. Check Membership (Admin OK)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Admin
        }]])
        // 3. Update Result (Returns Model)
        .append_query_results(vec![vec![channel::Model {
            id: chan_id, server_id: srv_id, name: "New".to_string(), description: "NewD".to_string(), position: 5
        }]])
        .into_connection();

    let (tx, _rx) = broadcast::channel(1);
    let req = UpdateChannelRequest { name: Some("New".to_string()), description: Some("NewD".to_string()), position: Some(5) };
    let res = channel_service::update_channel(&db, &tx, create_claims(user_id), chan_id, req).await;
    
    assert!(res.is_ok());
    let item = res.unwrap().channel;
    assert_eq!(item.name, "New");
    assert_eq!(item.position, 5);
}

#[tokio::test]
async fn test_update_channel_validations() {
    let chan_id = Uuid::new_v4();
    let srv_id = 55;
    let user_id = Uuid::new_v4();
    let (tx, _rx) = broadcast::channel(1);


    // Setup commun pour passer les checks DB initiaux
    let mock_setup = || {
        MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![channel::Model { id: chan_id, server_id: srv_id, name: "O".to_string(), description: "D".to_string(), position: 0 }]])
            .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner }]])
            .into_connection()
    };

    // 1. Empty Name
    let req1 = UpdateChannelRequest { name: Some("   ".to_string()), description: None, position: None };
    assert!(matches!(channel_service::update_channel(&mock_setup(), &tx, create_claims(user_id), chan_id, req1).await, Err(AppError::BadRequest(_))));

    // 2. Empty Description
    let req2 = UpdateChannelRequest { name: None, description: Some("".to_string()), position: None };
    assert!(matches!(channel_service::update_channel(&mock_setup(), &tx, create_claims(user_id), chan_id, req2).await, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_update_channel_insufficient_permissions() {
    let chan_id = Uuid::new_v4();
    let srv_id = 66;
    let user_id = Uuid::new_v4();
    let (tx, _rx) = broadcast::channel(1);


    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: chan_id, server_id: srv_id, name: "O".to_string(), description: "D".to_string(), position: 0 }]])
        // Membre standard (Pas Admin/Owner)
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member }]])
        .into_connection();

    let req = UpdateChannelRequest { name: Some("N".to_string()), description: None, position: None };
    let res = channel_service::update_channel(&db, &tx, create_claims(user_id), chan_id, req).await;
    assert!(matches!(res, Err(AppError::Forbidden(msg)) if msg.contains("Insufficient permissions")));
}

// Tests DB Errors (Update)
#[tokio::test]
async fn test_update_channel_db_errors() {
    let uid = Uuid::new_v4();
    let cid = Uuid::new_v4();
    let req = UpdateChannelRequest { name: Some("N".to_string()), description: None, position: None };
    let (tx, _rx) = broadcast::channel(1);


    // 1. Find Channel Fail
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("E1".to_string())])
        .into_connection();
    assert!(matches!(channel_service::update_channel(&db1, &tx, create_claims(uid), cid, req.clone()).await, Err(AppError::InternalServerError(_))));

    // 2. Find Member Fail
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_errors(vec![DbErr::Custom("E2".to_string())])
        .into_connection();
    assert!(matches!(channel_service::update_channel(&db2, &tx, create_claims(uid), cid, req.clone()).await, Err(AppError::InternalServerError(_))));

    // 3. Update Save Fail
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: uid, role: MemberRole::Owner }]])
        .append_query_errors(vec![DbErr::Custom("E3".to_string())])
        .into_connection();
    assert!(matches!(channel_service::update_channel(&db3, &tx, create_claims(uid), cid, req.clone()).await, Err(AppError::InternalServerError(_))));
}

// --- SUITE 3 : DELETE CHANNEL ---

#[tokio::test]
async fn test_delete_channel_success() {
    let chan_id = Uuid::new_v4();
    let srv_id = 77;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Channel
        .append_query_results(vec![vec![channel::Model { id: chan_id, server_id: srv_id, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        // 2. Check Perms (Admin)
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Admin }]])
        // 3. Delete Execution
        .append_exec_results(vec![MockExecResult { last_insert_id: 0, rows_affected: 1 }])
        .into_connection();

    let res = channel_service::delete_channel(&db, create_claims(user_id), chan_id).await;
    assert_eq!(res.unwrap(), StatusCode::OK);
}

// Tests DB Errors (Delete)
#[tokio::test]
async fn test_delete_channel_db_errors() {
    let uid = Uuid::new_v4();
    let cid = Uuid::new_v4();

    // 1. Find Channel Fail
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("E1".to_string())])
        .into_connection();
    assert!(matches!(channel_service::delete_channel(&db1, create_claims(uid), cid).await, Err(AppError::InternalServerError(_))));

    // 2. Find Member Fail
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_errors(vec![DbErr::Custom("E2".to_string())])
        .into_connection();
    assert!(matches!(channel_service::delete_channel(&db2, create_claims(uid), cid).await, Err(AppError::InternalServerError(_))));

    // 3. Delete Exec Fail
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: uid, role: MemberRole::Admin }]])
        .append_exec_errors(vec![DbErr::Custom("E3".to_string())])
        .into_connection();
    assert!(matches!(channel_service::delete_channel(&db3, create_claims(uid), cid).await, Err(AppError::InternalServerError(_))));
}