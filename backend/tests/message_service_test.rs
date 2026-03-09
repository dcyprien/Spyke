use sea_orm::{
    DatabaseBackend, MockDatabase, MockExecResult, DbErr, 
    EntityTrait, ActiveModelTrait
};
use uuid::Uuid;
use tokio::sync::broadcast;
use chrono::Utc;
use axum::http::StatusCode;

// Imports internes (ajustez selon votre structure exacte)
use backend::application::services::message_service;
use backend::application::dto::message_dto::SendMessageRequest;
use backend::application::dto::token_dto::Claims;
use backend::application::dto::apperror::AppError;
use backend::domain::models::{channel, message, server_member, server_member::MemberRole, user};

// Helper pour créer des claims
fn create_claims(user_id: Uuid) -> Claims {
    Claims {
        sub: user_id,
        username: "test_user".to_string(),
        exp: 10000000000,
        iat: 10000000000,
    }
}

// --- SUITE 1 : SEND MESSAGE (Happy Path & Validations) ---

#[tokio::test]
async fn test_send_message_success() {
    let (tx, _rx) = broadcast::channel(10);
    let user_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    let srv_id = 1;
    let msg_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Channel
        .append_query_results(vec![vec![channel::Model {
            id: channel_id, server_id: srv_id, name: "C".to_string(), description: "D".to_string(), position: 0
        }]])
        // 2. Check Membership
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        // 3. Insert Message
        .append_query_results(vec![vec![message::Model {
            id: msg_id, channel_id, server_id: srv_id, user_id, content: "Hello".to_string(), created_at: Utc::now()
        }]])
        // 4. Find User Info (pour le broadcast et return)
        .append_query_results(vec![vec![user::Model {
            id: user_id, username: "TheAuthor".to_string(), display_name: None, avatar_url: None, password_hash: "x".to_string(), status: backend::domain::models::user::UserStatus::Online
        }]])
        .into_connection();

    let req = SendMessageRequest { content: "Hello".to_string(), server_id: srv_id };
    
    let res = message_service::send_message(&db, &tx, create_claims(user_id), channel_id, req).await;
    
    assert!(res.is_ok());
    let item = res.unwrap();
    assert_eq!(item.content, "Hello");
    assert_eq!(item.author, "TheAuthor"); // Vérifie que l'author est bien populé
}

#[tokio::test]
async fn test_send_message_channel_mismatch() {
    let (tx, _rx) = broadcast::channel(1);
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Channel (Server ID = 1)
        .append_query_results(vec![vec![channel::Model {
            id: Uuid::new_v4(), server_id: 1, name: "C".to_string(), description: "D".to_string(), position: 0
        }]])
        .into_connection();

    // Request Server ID = 99 (Mismatch)
    let req = SendMessageRequest { content: "He".to_string(), server_id: 99 };
    let res = message_service::send_message(&db, &tx, create_claims(Uuid::new_v4()), Uuid::new_v4(), req).await;
    
    assert!(matches!(res, Err(AppError::BadRequest(msg)) if msg.contains("Channel error")));
}

#[tokio::test]
async fn test_send_message_not_member() {
    let (tx, _rx) = broadcast::channel(1);
    let srv_id = 1;
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: Uuid::new_v4(), server_id: srv_id, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        // 2. Check Membership -> Empty result
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .into_connection();

    let req = SendMessageRequest { content: "H".to_string(), server_id: srv_id };
    let res = message_service::send_message(&db, &tx, create_claims(Uuid::new_v4()), Uuid::new_v4(), req).await;
    
    assert!(matches!(res, Err(AppError::Forbidden(msg)) if msg.contains("Not a member")));
}

#[tokio::test]
async fn test_send_message_empty_content() {
    let (tx, _rx) = broadcast::channel(1);
    let srv_id = 1;
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: Uuid::new_v4(), server_id: srv_id, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: srv_id, user_id: Uuid::new_v4(), role: MemberRole::Member }]])
        .into_connection();

    let req = SendMessageRequest { content: "   ".to_string(), server_id: srv_id };
    let res = message_service::send_message(&db, &tx, create_claims(Uuid::new_v4()), Uuid::new_v4(), req).await;
    
    assert!(matches!(res, Err(AppError::BadRequest(msg)) if msg.contains("Empty content")));
}

// --- SUITE 2 : SEND MESSAGE (DB Errors - Coverage .map_err) ---

#[tokio::test]
async fn test_send_message_all_db_errors() {
    let (tx, _rx) = broadcast::channel(1);
    let uid = Uuid::new_v4();
    let cid = Uuid::new_v4();
    let req = SendMessageRequest { content: "msg".to_string(), server_id: 1 };

    // 1. Channel Find Error
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Find Channel Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::send_message(&db1, &tx, create_claims(uid), cid, req.clone()).await, Err(AppError::InternalServerError(_))));

    // 2. Membership Check Error
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_errors(vec![DbErr::Custom("Check Member Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::send_message(&db2, &tx, create_claims(uid), cid, req.clone()).await, Err(AppError::InternalServerError(_))));

    // 3. Insert Execution Error
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: uid, role: MemberRole::Member }]])
        .append_query_errors(vec![DbErr::Custom("Insert Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::send_message(&db3, &tx, create_claims(uid), cid, req.clone()).await, Err(AppError::InternalServerError(_))));

    // 4. User Lookup Error (Post insert)
    let db4 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: uid, role: MemberRole::Member }]])
        .append_query_results(vec![vec![message::Model { id: Uuid::new_v4(), channel_id: cid, server_id: 1, user_id: uid, content: "msg".to_string(), created_at: Utc::now() }]])
        .append_query_errors(vec![DbErr::Custom("Find User Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::send_message(&db4, &tx, create_claims(uid), cid, req.clone()).await, Err(AppError::InternalServerError(_))));
}

// --- SUITE 3 : GET MESSAGES (Logic + Mapping) ---

#[tokio::test]
async fn test_get_messages_mapping_success() {
    let cid = Uuid::new_v4();
    let uid = Uuid::new_v4();
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Channel
        .append_query_results(vec![vec![channel::Model { id: cid, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        // 2. Member
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: uid, role: MemberRole::Member }]])
        // 3. Messages + Joined User
        .append_query_results(vec![vec![
            // Msg 1 : Avec User
            (
                message::Model { id: Uuid::new_v4(), channel_id: cid, server_id: 1, user_id: uid, content: "A".to_string(), created_at: Utc::now() },
                Some(user::Model { id: uid, username: "Alice".to_string(), display_name: None, avatar_url: None, password_hash: "x".to_string(), status: backend::domain::models::user::UserStatus::Online })
            ),
            // Msg 2 : Sans User (supprimé)
            (
                message::Model { id: Uuid::new_v4(), channel_id: cid, server_id: 1, user_id: Uuid::new_v4(), content: "B".to_string(), created_at: Utc::now() },
                None 
            )
        ]])
        .into_connection();

    let res = message_service::get_messages(&db, create_claims(uid), cid).await;
    assert!(res.is_ok());
    let list = res.unwrap().message_list;
    
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].author, "Alice");
    assert_eq!(list[1].author, "Utilisateur Inconnu"); // Fallback coverage
}

#[tokio::test]
async fn test_get_messages_db_errors() {
    let uid = Uuid::new_v4();
    
    // 1. Channel Fail
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("X".to_string())])
        .into_connection();
    assert!(matches!(message_service::get_messages(&db1, create_claims(uid), Uuid::new_v4()).await, Err(AppError::InternalServerError(_))));

    // 2. Member Fail
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: Uuid::new_v4(), server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_errors(vec![DbErr::Custom("X".to_string())])
        .into_connection();
    assert!(matches!(message_service::get_messages(&db2, create_claims(uid), Uuid::new_v4()).await, Err(AppError::InternalServerError(_))));

    // 3. Message Fetch Fail (find_also_related error)
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![channel::Model { id: Uuid::new_v4(), server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: uid, role: MemberRole::Member }]])
        .append_query_errors(vec![DbErr::Custom("X".to_string())])
        .into_connection();
    assert!(matches!(message_service::get_messages(&db3, create_claims(uid), Uuid::new_v4()).await, Err(AppError::InternalServerError(_))));
}

// --- SUITE 4 : DELETE MESSAGE (Permissions & Logic) ---

#[tokio::test]
async fn test_delete_own_message_success() {
    let msg_id = Uuid::new_v4();
    let author_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Message (User matches request)
        .append_query_results(vec![vec![message::Model {
            id: msg_id, channel_id: Uuid::new_v4(), server_id: 1, user_id: author_id, content: "M".to_string(), created_at: Utc::now()
        }]])
        // 2. Delete
        .append_exec_results(vec![MockExecResult { last_insert_id: 0, rows_affected: 1 }])
        .into_connection();

    let (tx, _rx) = broadcast::channel(1);
    let res = message_service::delete_message(&db, &tx, create_claims(author_id), msg_id).await;
    assert_eq!(res.unwrap(), StatusCode::OK);
}

#[tokio::test]
async fn test_delete_other_message_as_owner() {
    let msg_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let other_user = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    let srv_id = 99;

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Message (Author != Requester)
        .append_query_results(vec![vec![message::Model {
            id: msg_id, channel_id, server_id: srv_id, user_id: other_user, content: "M".to_string(), created_at: Utc::now()
        }]])
        // 2. Find Channel (pour retrouver le server_id)
        .append_query_results(vec![vec![channel::Model {
            id: channel_id, server_id: srv_id, name: "C".to_string(), description: "D".to_string(), position: 0
        }]])
        // 3. Find Membership (Requester is Owner)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Owner
        }]])
        // 4. Delete
        .append_exec_results(vec![MockExecResult { last_insert_id: 0, rows_affected: 1 }])
        .into_connection();

    let (tx, _rx) = broadcast::channel(1);
    let res = message_service::delete_message(&db, &tx, create_claims(owner_id), msg_id).await;
    assert_eq!(res.unwrap(), StatusCode::OK);
}

#[tokio::test]
async fn test_delete_other_message_forbidden() {
    // Cas où un simple membre essaie de supprimer le message d'un autre
    let msg_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![message::Model { id: msg_id, channel_id, server_id: 1, user_id: Uuid::new_v4(), content: "M".to_string(), created_at: Utc::now() }]])
        .append_query_results(vec![vec![channel::Model { id: channel_id, server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id, role: MemberRole::Member }]]) // Rôle insuffisant
        .into_connection();

    let (tx, _rx) = broadcast::channel(1);
    let res = message_service::delete_message(&db, &tx, create_claims(user_id), msg_id).await;
    assert!(matches!(res, Err(AppError::Forbidden(msg)) if msg.contains("Permission denied")));
}

// --- SUITE 5 : DELETE MESSAGE (DB Errors .map_err) ---

#[tokio::test]
async fn test_delete_message_db_errors() {
    let mid = Uuid::new_v4();
    let uid = Uuid::new_v4();

    // 1. Initial Message Find Fail
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Find Msg Fail".to_string())])
        .into_connection();
    let (tx, _rx) = broadcast::channel(1);
    assert!(matches!(message_service::delete_message(&db1, &tx, create_claims(uid), mid).await, Err(AppError::InternalServerError(_))));

    // 2. Author Delete Exec Fail (Case: Own message)
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![message::Model { id: mid, channel_id: Uuid::new_v4(), server_id: 1, user_id: uid, content: "M".to_string(), created_at: Utc::now() }]])
        .append_exec_errors(vec![DbErr::Custom("Delete Exec Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::delete_message(&db2, &tx, create_claims(uid), mid).await, Err(AppError::InternalServerError(_))));

    // 3. Channel Fetch Fail (Case: Not own message)
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![message::Model { id: mid, channel_id: Uuid::new_v4(), server_id: 1, user_id: Uuid::new_v4(), content: "M".to_string(), created_at: Utc::now() }]])
        .append_query_errors(vec![DbErr::Custom("Find Channel Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::delete_message(&db3,  &tx, create_claims(uid), mid).await, Err(AppError::InternalServerError(_))));

    // 4. Membership Fetch Fail
    let db4 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![message::Model { id: mid, channel_id: Uuid::new_v4(), server_id: 1, user_id: Uuid::new_v4(), content: "M".to_string(), created_at: Utc::now() }]])
        .append_query_results(vec![vec![channel::Model { id: Uuid::new_v4(), server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_errors(vec![DbErr::Custom("Find Membership Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::delete_message(&db4, &tx, create_claims(uid), mid).await, Err(AppError::InternalServerError(_))));

    // 5. Admin Delete Exec Fail (Case: Admin)
    let db5 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![message::Model { id: mid, channel_id: Uuid::new_v4(), server_id: 1, user_id: Uuid::new_v4(), content: "M".to_string(), created_at: Utc::now() }]])
        .append_query_results(vec![vec![channel::Model { id: Uuid::new_v4(), server_id: 1, name: "C".to_string(), description: "".to_string(), position: 0 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: uid, role: MemberRole::Admin }]])
        .append_exec_errors(vec![DbErr::Custom("Delete Admin Fail".to_string())])
        .into_connection();
    assert!(matches!(message_service::delete_message(&db5, &tx, create_claims(uid), mid).await, Err(AppError::InternalServerError(_))));
}