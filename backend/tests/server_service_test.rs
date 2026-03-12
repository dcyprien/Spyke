use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseBackend, DbErr, EntityTrait, MockDatabase, 
    MockExecResult
};
use uuid::Uuid;
use tokio::sync::broadcast;
use serde_json::Value;

use backend::application::services::server_service;
use backend::application::dto::server_dto::{
    CreateServerRequest, UpdateServerRequest, UpdateMemberRequest, 
    JoinServerRequest, CreateChannelRequest
};
use backend::application::dto::token_dto::Claims;
use backend::application::dto::apperror::AppError;
use backend::domain::models::{server_ban, server_model, server_member, channel, user};
use backend::domain::models::server_member::MemberRole;
use backend::domain::models::user::UserStatus;

// --- UTILS ---

fn create_claims(user_id: Uuid) -> Claims {
    Claims {
        sub: user_id,
        username: "test_user".to_string(),
        exp: 9999999999,
        iat: 0,
    }
}

// --- SUITE 1 : CREATE SERVER ---

#[tokio::test]
async fn test_create_server_success() {
    let user_id = Uuid::new_v4();
    let server_id = 100;
    
    // 1. Check Name (Empty result = OK)
    // 2. Insert Server (Return Model)
    // 3. Insert Owner Membership (Return Model)
    // 4. Insert Updated Default Channel (Return Model)
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<server_model::Model>]) 
        .append_query_results(vec![vec![server_model::Model {
            id: server_id,
            name: "My Server".to_string(),
            description: "Desc".to_string(),
            icon_url: None,
            owner_id: user_id,
            invitcode: 1234,
        }]])
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id, user_id, role: MemberRole::Owner
        }]])
        .append_query_results(vec![vec![channel::Model {
            id: Uuid::new_v4(), server_id, name: "general".to_string(), description: "gen".to_string(), position: 0
        }]])
        .into_connection();

    let req = CreateServerRequest { 
        name: "My Server".to_string(), 
        description: "Desc".to_string(), 
        icon_url: None 
    };

    let res = server_service::create_server(&db, create_claims(user_id), req).await;
    
    assert!(res.is_ok());
    let srv = res.unwrap();
    assert_eq!(srv.name, "My Server");
    assert_eq!(srv.channels.len(), 1);
    assert_eq!(srv.channels[0].name, "general");
}

#[tokio::test]
async fn test_create_server_name_duplicate() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model {
            id: 1, name: "Taken".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1
        }]])
        .into_connection();

    let req = CreateServerRequest { name: "Taken".to_string(), description: "D".to_string(), icon_url: None };
    let res = server_service::create_server(&db, create_claims(Uuid::new_v4()), req).await;

    assert!(matches!(res, Err(AppError::BadRequest(msg)) if msg == "Server's name already in use"));
}

// --- SUITE 2 : GET SERVERS (COMPLEX FLOW) ---

#[tokio::test]
async fn test_get_servers_complex_flow() {
    let user_id = Uuid::new_v4();
    let server_id = 99;

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find Membership + Related Server
        .append_query_results(vec![vec![
            (
                server_member::Model { id: Uuid::new_v4(), server_id, user_id, role: MemberRole::Owner },
                Some(server_model::Model { id: server_id, name: "S1".to_string(), description: "D".to_string(), icon_url: None, owner_id: user_id, invitcode: 111 })
            )
        ]])
        // 2. Find All Channels (WHERE server_id IN [...])
        .append_query_results(vec![vec![
            channel::Model { id: Uuid::new_v4(), server_id, name: "C1".to_string(), description: "D".to_string(), position: 0 }
        ]])
        // 3. Find All Members (WHERE server_id IN [...]) + User JOIN
        .append_query_results(vec![vec![
            (
                server_member::Model { id: Uuid::new_v4(), server_id, user_id, role: MemberRole::Owner },
                Some(user::Model { id: user_id, username: "Me".to_string(), display_name: None, avatar_url: None, password_hash: "h".to_string(), status: UserStatus::Online })
            )
        ]])
        .into_connection();

    let res = server_service::get_servers(&db, create_claims(user_id)).await;
    
    assert!(res.is_ok());
    let list = res.unwrap().server_list;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, server_id);
    assert_eq!(list[0].channels.len(), 1);
    assert_eq!(list[0].members.len(), 1);
    assert_eq!(list[0].members[0].username, "Me");
}

// --- SUITE 3 : JOIN SERVER + WEBSOCKET ---

#[tokio::test]
async fn test_join_server_success_with_broadcast() {
    let user_id = Uuid::new_v4();
    let server_id = 50;
    
    // Broadcast Setup
    let (tx, mut rx) = broadcast::channel(1);

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Verif Server Existing
        .append_query_results(vec![vec![server_model::Model {
            id: server_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 9999
        }]])
        // 2. Ban Check (Not Banned)
        .append_query_results(vec![vec![] as Vec<server_ban::Model>])
        // 3. Verif Not Member (Empty)
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // 4. Insert Membership
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id, user_id, role: MemberRole::Member
        }]])
        // 4. Find User Info (For Broadcast)
        .append_query_results(vec![vec![user::Model {
            id: user_id, username: "Joiner".to_string(), display_name: None, avatar_url: None, password_hash: "h".to_string(), status: UserStatus::Online
        }]])
        // 5. Fetch Channels (For Response)
        .append_query_results(vec![vec![channel::Model {
            id: Uuid::new_v4(), server_id, name: "Welcome".to_string(), description: "".to_string(), position: 0
        }]])
        // 6. Fetch Members (For Response)
        .append_query_results(vec![vec![
            (
                server_member::Model { id: Uuid::new_v4(), server_id, user_id, role: MemberRole::Member },
                Some(user::Model { id: user_id, username: "Joiner".to_string(), display_name: None, avatar_url: None, password_hash: "h".to_string(), status: UserStatus::Online })
            )
        ]])
        .into_connection();

    let req = JoinServerRequest { invitcode: 9999 };
    let res = server_service::join_server(&db, &tx, create_claims(user_id), server_id, req).await;

    assert!(res.is_ok());

    // Vérification du WebSocket
    let ws_msg = rx.recv().await.unwrap();
    let json: Value = serde_json::from_str(&ws_msg).unwrap();
    assert_eq!(json["type"], "user_joined");
    assert_eq!(json["data"]["member"]["username"], "Joiner");
}

#[tokio::test]
async fn test_join_server_wrong_code() {
    let (tx, _) = broadcast::channel(1);
    let srv_id = 1;
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Server Found (code 1234)
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1234
        }]])
        // 2. Ban Check (Not Banned)
        .append_query_results(vec![vec![] as Vec<server_ban::Model>])
        // 3. Not Member
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .into_connection();

    let req = JoinServerRequest { invitcode: 0000 };
    let res = server_service::join_server(&db, &tx, create_claims(Uuid::new_v4()), srv_id, req).await;

    assert!(matches!(res, Err(AppError::BadRequest(msg)) if msg == "Invalid invitation code"));
}

// --- SUITE 4 : SERVER MANAGEMENT (UPDATE / DELETE) ---

#[tokio::test]
async fn test_update_server_forbidden_role() {
    let srv_id = 10;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Membership Check -> Member Role (Not Owner/Admin)
        .append_query_results(vec![vec![server_member::Model{
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        .into_connection();

    let req = UpdateServerRequest { 
        id: srv_id, 
        name: Some("N".to_string()), 
        description: None, 
        icon_url: None 
    };
    
    let res = server_service::update_server(&db, create_claims(user_id), srv_id, req).await;
    
    assert!(matches!(res, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn test_delete_server_not_owner() {
    let srv_id = 55;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Membership Check -> Admin (Admin cannot delete server)
        .append_query_results(vec![vec![server_member::Model{
             id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Admin
        }]])
        .into_connection();

    let res = server_service::delete_server(&db, create_claims(user_id), srv_id).await;
    assert!(matches!(res, Err(AppError::Forbidden(msg)) if msg.contains("Only owners")));
}

#[tokio::test]
async fn test_leave_server_as_owner() {
    let srv_id = 66;
    let user_id = Uuid::new_v4();
    let (tx, _rx) = broadcast::channel(1);
    let db = MockDatabase::new(DatabaseBackend::Postgres)

        // 1. Server Exists
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: user_id, invitcode: 1
        }]])
        // 2. Membership -> Owner
        .append_query_results(vec![vec![server_member::Model{
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner
       }]])
       .into_connection();

    let res = server_service::leave_server(&db, &tx, create_claims(user_id), srv_id).await;
    assert!(matches!(res, Err(AppError::Forbidden(msg)) if msg.contains("Owner cannot leave")));
}

// --- SUITE 5 : MEMBER MANAGEMENT & TRANSFER ---

#[tokio::test]
async fn test_update_member_promote_admin() {
    let (tx, mut rx) = broadcast::channel(1);
    let srv_id = 1;
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Requester is Owner
        .append_query_results(vec![vec![server_member::Model {
             id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Owner
        }]])
        // 2. Target exists
        .append_query_results(vec![vec![server_member::Model {
             id: Uuid::new_v4(), server_id: srv_id, user_id: target_id, role: MemberRole::Member
        }]])
        // 3. Update Target Role (Success)
        .append_query_results(vec![vec![server_member::Model {
             id: Uuid::new_v4(), server_id: srv_id, user_id: target_id, role: MemberRole::Admin
        }]])
        // 4. Find User Info (For Return & Broadcast)
        .append_query_results(vec![vec![user::Model {
            id: target_id, username: "Target".to_string(), display_name: None, avatar_url: None, password_hash: "h".to_string(), status: UserStatus::Online
        }]])
        .into_connection();

    let req = UpdateMemberRequest { new_role: "admin".to_string() };
    let res = server_service::update_member(&db, &tx, create_claims(owner_id), srv_id, target_id, req).await;

    assert!(res.is_ok());
    // Verify Broadcast
    let msg = rx.recv().await.unwrap();
    assert!(msg.contains("member_updated"));
    assert!(msg.contains("Admin"));
}

#[tokio::test]
async fn test_update_member_transfer_ownership() {
    let (tx, mut rx) = broadcast::channel(10); // Queue > 1 car 2 messages
    let srv_id = 200;
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Requester is Owner
        .append_query_results(vec![vec![server_member::Model {
             id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Owner
        }]])
        // 2. Target found
        .append_query_results(vec![vec![server_member::Model {
             id: Uuid::new_v4(), server_id: srv_id, user_id: target_id, role: MemberRole::Member
        }]])
        // 3. Find Server
        .append_query_results(vec![vec![server_model::Model {
             id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id, invitcode: 1
        }]])
        // 4. Update Server Owner -> Target (Return Updated)
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: target_id, invitcode: 1
        }]])
        // 5. Demote Requester -> Admin (Return Updated)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Admin
        }]])
        // 6. Find Old Owner User Info (For Broadcast 1)
        .append_query_results(vec![vec![user::Model {
            id: owner_id, username: "OldBoss".to_string(), display_name: None, avatar_url: None, password_hash: "h".to_string(), status: UserStatus::Online
        }]])
        // 7. Update Target -> Owner (Return Updated)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id: target_id, role: MemberRole::Owner
        }]])
        // 8. Find New Owner User Info (For Broadcast 2 & Return)
        .append_query_results(vec![vec![user::Model {
            id: target_id, username: "NewBoss".to_string(), display_name: None, avatar_url: None, password_hash: "h".to_string(), status: UserStatus::Online
        }]])
        .into_connection();

    let req = UpdateMemberRequest { new_role: "owner".to_string() };
    let res = server_service::update_member(&db, &tx, create_claims(owner_id), srv_id, target_id, req).await;

    assert!(res.is_ok());

    // Vérifier les 2 broadcasts
    let msg1 = rx.recv().await.unwrap(); // Old Owner -> Admin
    assert!(msg1.contains("OldBoss"));
    assert!(msg1.contains("Admin"));

    let msg2 = rx.recv().await.unwrap(); // New Owner -> Owner
    assert!(msg2.contains("NewBoss"));
    assert!(msg2.contains("Owner"));
}

// --- SUITE 6 : CHANNELS + WEBSOCKET ---

#[tokio::test]
async fn test_create_channel_broadcast() {
    let (tx, mut rx) = broadcast::channel(1);
    let srv_id = 77;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Perms (Admin is allowed)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Admin
        }]])
        // 2. Insert Channel
        .append_query_results(vec![vec![channel::Model {
            id: Uuid::new_v4(), server_id: srv_id, name: "gaming".to_string(), description: "".to_string(), position: 0
        }]])
        .into_connection();

    let req = CreateChannelRequest { name: "gaming".to_string(), description: "".to_string() };
    let res = server_service::create_channel(&db, &tx, create_claims(user_id), srv_id, req).await;

    assert!(res.is_ok());

    // Broadcast check
    let msg = rx.recv().await.unwrap();
    let json: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(json["type"], "channel_created");
    assert_eq!(json["data"]["channel"]["name"], "gaming");
}

#[tokio::test]
async fn test_get_channels_success() {
    let srv_id = 78;
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Membership
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        // 2. Channels
        .append_query_results(vec![vec![
            channel::Model { id: Uuid::new_v4(), server_id: srv_id, name: "c1".to_string(), description: "".to_string(), position: 0},
            channel::Model { id: Uuid::new_v4(), server_id: srv_id, name: "c2".to_string(), description: "".to_string(), position: 0}
        ]])
        .into_connection();

    let res = server_service::get_channels(&db, create_claims(user_id), srv_id).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().channels.len(), 2);
}

#[tokio::test]
async fn test_get_channels_forbidden() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<server_member::Model>]) // Not member
        .into_connection();

    let res = server_service::get_channels(&db, create_claims(Uuid::new_v4()), 1).await;
    assert!(matches!(res, Err(AppError::Forbidden(_))));
}

// --- SUITE 7 : DB ERROR INJECTION ---

#[tokio::test]
async fn test_create_server_db_crash() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<server_model::Model>]) // Found Name OK
        .append_query_errors(vec![DbErr::Custom("Insert Fail".to_string())])
        .into_connection();

    let req = CreateServerRequest { name: "Crash".to_string(), description: "D".to_string(), icon_url: None };
    let res = server_service::create_server(&db, create_claims(Uuid::new_v4()), req).await;
    
    assert!(matches!(res, Err(AppError::InternalServerError(msg)) if msg.contains("Insert Fail")));
}

// --- SUITE 8 : VALIDATIONS & EDGE CASES (Pour le 100% Coverage) ---

#[tokio::test]
async fn test_create_server_validations() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // Le service vérifie d'abord l'unicité du nom en DB avant les validations de format
        .append_query_results(vec![vec![] as Vec<server_model::Model>]) // Nom unique
        .append_query_results(vec![vec![] as Vec<server_model::Model>]) // Nom unique (pour le 2eme test)
        .append_query_results(vec![vec![] as Vec<server_model::Model>]) // Nom unique (pour le 3eme test)
        .into_connection();

    // 1. Nom vide
    let req1 = CreateServerRequest { name: "".to_string(), description: "d".to_string(), icon_url: None };
    let res1 = server_service::create_server(&db, create_claims(Uuid::new_v4()), req1).await;
    assert!(matches!(res1, Err(AppError::BadRequest(msg)) if msg.contains("cannot be empty")));

    // 2. Nom trop long (>20 chars)
    let req2 = CreateServerRequest { name: "a".repeat(21), description: "d".to_string(), icon_url: None };
    let res2 = server_service::create_server(&db, create_claims(Uuid::new_v4()), req2).await;
    assert!(matches!(res2, Err(AppError::BadRequest(msg)) if msg.contains("too long")));

    // 3. Description vide
    let req3 = CreateServerRequest { name: "valid".to_string(), description: "   ".to_string(), icon_url: None };
    let res3 = server_service::create_server(&db, create_claims(Uuid::new_v4()), req3).await;
    assert!(matches!(res3, Err(AppError::BadRequest(msg)) if msg.contains("Description required")));
}

#[tokio::test]
async fn test_join_server_already_member() {
    let (tx, _rx) = broadcast::channel(1);
    let srv_id = 99;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Server Exists
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1111
        }]])
        // 2. Ban Check (Not Banned)
        .append_query_results(vec![vec![] as Vec<server_ban::Model>])
        // 3. Check Existing Membership -> FOUND (Déjà membre)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        .into_connection();

    let req = JoinServerRequest { invitcode: 1111 };
    let res = server_service::join_server(&db, &tx, create_claims(user_id), srv_id, req).await;

    // Doit retourner BadRequest car déjà membre
    assert!(matches!(res, Err(AppError::BadRequest(msg)) if msg.contains("Already a member")));
}

#[tokio::test]
async fn test_update_server_validations_and_icon() {
    let srv_id = 10;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // --- CAS 1 : Nom vide ---
        // 1. Membership Check
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner }]])
        // 2. Get Server
        .append_query_results(vec![vec![server_model::Model { id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: user_id, invitcode: 1 }]])
        
        // --- CAS 2 : Update Icon URL (Succès) ---
        // 1. Membership Check
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner }]])
        // 2. Get Server
        .append_query_results(vec![vec![server_model::Model { id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: user_id, invitcode: 1 }]])
        // 3. Update Result
        .append_query_results(vec![vec![server_model::Model { id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: Some("http://icon.png".to_string()), owner_id: user_id, invitcode: 1 }]])
        
        .into_connection();

    // Test 1: Validation Nom Vide
    let req1 = UpdateServerRequest { id: srv_id, name: Some("".to_string()), description: None, icon_url: None };
    let res1 = server_service::update_server(&db, create_claims(user_id), srv_id, req1).await;
    assert!(matches!(res1, Err(AppError::BadRequest(msg)) if msg.contains("cannot be empty")));

    // Test 2: Update Icon URL (Couvre la ligne `if let Some(icon_url)`)
    let req2 = UpdateServerRequest { id: srv_id, name: None, description: None, icon_url: Some("http://icon.png".to_string()) };
    let res2 = server_service::update_server(&db, create_claims(user_id), srv_id, req2).await;
    assert!(res2.is_ok());
    assert_eq!(res2.unwrap().new_icon_url, Some("http://icon.png".to_string()));
}

#[tokio::test]
async fn test_update_member_edge_cases() {
    let (tx, _rx) = broadcast::channel(1);
    let srv_id = 50;
    let owner_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // --- CAS 1 : Modifier son propre rôle ---
        // 1. Requester Membership
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Owner }]])
        // 2. Target Membership (Soi-même)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Owner }]])

        // --- CAS 2 : Role invalide ---
        // 1. Requester
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Owner }]])
        // 2. Target (Autre user)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id: Uuid::new_v4(), role: MemberRole::Member }]])

        .into_connection();

    // Test 1: Self update forbidden
    // On passe owner_id comme target ET comme claim
    let req1 = UpdateMemberRequest { new_role: "admin".to_string() };
    let res1 = server_service::update_member(&db, &tx, create_claims(owner_id), srv_id, owner_id, req1).await;
    assert!(matches!(res1, Err(AppError::Forbidden(msg)) if msg.contains("Cannot modify your own role")));

    // Test 2: Invalid Role string
    let req2 = UpdateMemberRequest { new_role: "god_mode".to_string() };
    let res2 = server_service::update_member(&db, &tx, create_claims(owner_id), srv_id, Uuid::new_v4(), req2).await;
    assert!(matches!(res2, Err(AppError::Forbidden(msg)) if msg.contains("Invalid role")));
}

#[tokio::test]
async fn test_create_channel_empty_name() {
    let (tx, _rx) = broadcast::channel(1);
    let srv_id = 70;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Perms
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Admin
        }]])
        .into_connection();

    let req = CreateChannelRequest { name: "   ".to_string(), description: "d".to_string() };
    let res = server_service::create_channel(&db, &tx, create_claims(user_id), srv_id, req).await;

    assert!(matches!(res, Err(AppError::BadRequest(msg)) if msg.contains("cannot be empty")));
}

// --- SUITE 9 : MISSING HAPPY PATHS (Delete & Leave) ---

#[tokio::test]
async fn test_delete_server_success() {
    let srv_id = 555;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Membership (Owner)
        .append_query_results(vec![vec![server_member::Model{
             id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner
        }]])
        // 2. AJOUT : Get Server (Le service vérifie que le serveur existe avant de supprimer)
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: user_id, invitcode: 1
        }]])
        // 3. Delete Execution
        .append_exec_results(vec![
             MockExecResult { last_insert_id: 0, rows_affected: 1 }
        ])
        .into_connection();

    let res = server_service::delete_server(&db, create_claims(user_id), srv_id).await;
    
    // Debug: si ça fail encore, on affiche l'erreur
    if let Err(e) = &res {
        println!("Delete failed with: {:?}", e);
    }
    assert!(res.is_ok(), "La suppression devrait réussir pour le propriétaire");
}

#[tokio::test]
async fn test_leave_server_success() {
    let srv_id = 666;
    let user_id = Uuid::new_v4();
    let (tx, _rx) = broadcast::channel(1);
    let db = MockDatabase::new(DatabaseBackend::Postgres)

        // 1. Server Exists
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1
        }]])
        // 2. Membership -> Member (Pas Owner, donc peut partir)
        .append_query_results(vec![vec![server_member::Model{
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
       }]])
       // 3. Delete Execution
       .append_exec_results(vec![
            MockExecResult { last_insert_id: 0, rows_affected: 1 }
       ])
       .into_connection();

    let res = server_service::leave_server(&db, &tx, create_claims(user_id), srv_id).await;
    assert!(res.is_ok(), "Un membre standard devrait pouvoir quitter le serveur");
}

// --- SUITE 10 : GETTERS (Coverage Manquant) ---

#[tokio::test]
async fn test_get_server_by_id_success() {
    let srv_id = 111;
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Membership Check
        .append_query_results(vec![vec![server_member::Model { 
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member 
        }]])
        // 2. Get Server
        .append_query_results(vec![vec![server_model::Model { 
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 
        }]])
        // 3. Get Channels
        .append_query_results(vec![vec![channel::Model { 
            id: Uuid::new_v4(), server_id: srv_id, name: "C".to_string(), description: "D".to_string(), position: 0 
        }]])
        // 4. Get Members
        .append_query_results(vec![vec![(
            server_member::Model { id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member },
            Some(user::Model { id: user_id, username: "U".to_string(), display_name: None, avatar_url: None, password_hash: "x".to_string(), status: UserStatus::Online })
        )]])
        .into_connection();

    let res = server_service::get_server_by_id(&db, create_claims(user_id), srv_id).await;
    assert!(res.is_ok());
    let data = res.unwrap().server;
    assert!(!data.channels.is_empty());
}

#[tokio::test]
async fn test_get_servermembers_success() {
    let srv_id = 200;
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Membership
        .append_query_results(vec![vec![server_member::Model { 
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member 
        }]])
        // 2. Fetch Members
        .append_query_results(vec![vec![(
            server_member::Model { id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member },
            Some(user::Model { id: user_id, username: "U".to_string(), display_name: None, avatar_url: None, password_hash: "x".to_string(), status: UserStatus::Online })
        )]])
        .into_connection();

    let res = server_service::get_servermembers(&db, create_claims(user_id), srv_id).await;
    assert!(res.is_ok());
}

// --- SUITE 11 : ERROR PROPAGATION (DB ERRORS) & DATA MAPPING (100% Target) ---

// 1. CREATE SERVER ERRORS
#[tokio::test]
async fn test_create_server_name_check_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Name Check Fail".to_string())])
        .into_connection();
    let req = CreateServerRequest { name: "Fail".to_string(), description: "D".to_string(), icon_url: None };
    let res = server_service::create_server(&db, create_claims(Uuid::new_v4()), req).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_create_server_insert_server_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<server_model::Model>])
        .append_query_errors(vec![DbErr::Custom("Insert Server Fail".to_string())])
        .into_connection();
    let req = CreateServerRequest { name: "Fail".to_string(), description: "D".to_string(), icon_url: None };
    let res = server_service::create_server(&db, create_claims(Uuid::new_v4()), req).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_create_server_insert_member_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<server_model::Model>]) // Name Check
        .append_query_results(vec![vec![server_model::Model { id: 1, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 }]]) // Insert Srv OK
        .append_query_errors(vec![DbErr::Custom("Insert Member Fail".to_string())])
        .into_connection();
    let req = CreateServerRequest { name: "Fail".to_string(), description: "D".to_string(), icon_url: None };
    let res = server_service::create_server(&db, create_claims(Uuid::new_v4()), req).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_create_server_insert_channel_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<server_model::Model>])
        .append_query_results(vec![vec![server_model::Model { id: 1, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 }]])
        .append_query_results(vec![vec![server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id: Uuid::new_v4(), role: MemberRole::Owner }]])
        .append_query_errors(vec![DbErr::Custom("Insert Channel Fail".to_string())])
        .into_connection();
    let req = CreateServerRequest { name: "Fail".to_string(), description: "D".to_string(), icon_url: None };
    let res = server_service::create_server(&db, create_claims(Uuid::new_v4()), req).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

// 2. GET SERVERS ERRORS
#[tokio::test]
async fn test_get_servers_db_errors() {
    let user_id = Uuid::new_v4();

    // Case A: Fetch Memberships Fail (Line 89)
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Fetch Memberships Fail".to_string())])
        .into_connection();
    let res = server_service::get_servers(&db, create_claims(user_id)).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))));

    // Case B: Fetch Channels Fail (Line 102)
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![ (server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id, role: MemberRole::Owner }, None::<server_model::Model>) ]])
        .append_query_errors(vec![DbErr::Custom("Fetch Channels Fail".to_string())])
        .into_connection();
    let res2 = server_service::get_servers(&db2, create_claims(user_id)).await;
    assert!(matches!(res2, Err(AppError::InternalServerError(_))));

    // Case C: Fetch Members Fail (Line 110)
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![ (server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id, role: MemberRole::Owner }, None::<server_model::Model>) ]])
        .append_query_results(vec![vec![] as Vec<channel::Model>])
        .append_query_errors(vec![DbErr::Custom("Fetch Members Fail".to_string())])
        .into_connection();
    let res3 = server_service::get_servers(&db3, create_claims(user_id)).await;
    assert!(matches!(res3, Err(AppError::InternalServerError(_))));
}

// 3. GET SERVER BY ID
#[tokio::test]
async fn test_get_server_by_id_db_errors() {
    let srv_id = 1;
    let user_id = Uuid::new_v4();

    // Line 167: Membership Check
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Check Membership Fail".to_string())])
        .into_connection();
    assert!(matches!(server_service::get_server_by_id(&db1, create_claims(user_id), srv_id).await, Err(AppError::InternalServerError(_))));

    // Line 176: Find Server
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member }]])
        .append_query_errors(vec![DbErr::Custom("Find Server Fail".to_string())])
        .into_connection();
    assert!(matches!(server_service::get_server_by_id(&db2, create_claims(user_id), srv_id).await, Err(AppError::InternalServerError(_))));

    // Line 185: Find Channels
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member }]])
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 }]])
        .append_query_errors(vec![DbErr::Custom("Find Channels Fail".to_string())])
        .into_connection();
    assert!(matches!(server_service::get_server_by_id(&db3, create_claims(user_id), srv_id).await, Err(AppError::InternalServerError(_))));

    // Line 202: Find Members
    let db4 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member }]])
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 }]])
        .append_query_results(vec![vec![] as Vec<channel::Model>])
        .append_query_errors(vec![DbErr::Custom("Find Members Fail".to_string())])
        .into_connection();
    assert!(matches!(server_service::get_server_by_id(&db4, create_claims(user_id), srv_id).await, Err(AppError::InternalServerError(_))));
}

// 4. UPDATE SERVER ERRORS
#[tokio::test]
async fn test_update_server_db_errors() {
    let srv_id = 99;
    let user_id = Uuid::new_v4();
    let req = UpdateServerRequest { id: srv_id, name: Some("N".to_string()), description: None, icon_url: None };

    // Line 238: Check Membership
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Auth DB Error".to_string())])
        .into_connection();
    assert!(matches!(server_service::update_server(&db1, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));

    // Line 249: Find Server
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner }]])
        .append_query_errors(vec![DbErr::Custom("Find Server Error".to_string())])
        .into_connection();
    assert!(matches!(server_service::update_server(&db2, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));

    // Line 282: Update Execute
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner }]])
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: user_id, invitcode: 1 }]])
        .append_query_errors(vec![DbErr::Custom("Update Execute Error".to_string())])
        .into_connection();
    assert!(matches!(server_service::update_server(&db3, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));
}

// 5. DELETE SERVER ERRORS
#[tokio::test]
async fn test_delete_server_member_check_error() {
    let srv_id = 88;
    let user_id = Uuid::new_v4();

    // Cas 1 : Erreur lors de la vérification du membre
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Check Member Fail".to_string())])
        .into_connection();

    let res = server_service::delete_server(&db, create_claims(user_id), srv_id).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))), "Should error on Member Check DB fail");
}

#[tokio::test]
async fn test_delete_server_fetch_error() {
    let srv_id = 89;
    let user_id = Uuid::new_v4();

    // Cas 2 : Erreur lors de la récupération du serveur (qui doit exister avant delete)
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Perms (OK)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner }]])
        // 2. Find Server (FAIL)
        .append_query_errors(vec![DbErr::Custom("Find Server Fail".to_string())])
        .into_connection();

    let res = server_service::delete_server(&db, create_claims(user_id), srv_id).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))), "Should error on Find Server DB fail");
}

#[tokio::test]
async fn test_delete_server_execution_error() {
    let srv_id = 90;
    let user_id = Uuid::new_v4();

    // Cas 3 : Erreur lors de l'exécution du DELETE
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Perms (OK)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner }]])
        // 2. Find Server (OK)
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: user_id, invitcode: 1 }]])
        // 3. Exec Delete (FAIL)
        .append_exec_errors(vec![DbErr::Custom("Delete Exec Error".to_string())])
        .into_connection();

    let res = server_service::delete_server(&db, create_claims(user_id), srv_id).await;
    
    // Debug explicite pour comprendre pourquoi ça fail si ça fail
    match &res {
        Ok(_) => panic!("❌ Delete Server succeeded but DB Error was expected! Does logic ignore the delete error?"),
        Err(AppError::InternalServerError(msg)) => println!("✅ Correctly caught DB error: {}", msg),
        Err(e) => panic!("❌ Expected InternalServerError but got: {:?}", e),
    }
}

// 6. JOIN SERVER ERRORS
#[tokio::test]
async fn test_join_server_db_errors() {
    let (tx, _rx) = broadcast::channel(1);
    let srv_id = 77;
    let user_id = Uuid::new_v4();
    let req = JoinServerRequest { invitcode: 1234 };

    // Line 331: Find Server
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("E1".to_string())])
        .into_connection();
    assert!(matches!(server_service::join_server(&db1, &tx, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));

    // Line 340: Ban Check Error
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1234 }]])
        .append_query_errors(vec![DbErr::Custom("E2".to_string())])
        .into_connection();
    assert!(matches!(server_service::join_server(&db2, &tx, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));

    // Line 359: Check Already Member Error
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1234 }]])
        .append_query_results(vec![vec![] as Vec<server_ban::Model>])
        .append_query_errors(vec![DbErr::Custom("E3".to_string())])
        .into_connection();
    assert!(matches!(server_service::join_server(&db3, &tx, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));

    // Line 366: Insert Member Error
    let db4 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1234 }]])
        .append_query_results(vec![vec![] as Vec<server_ban::Model>])
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .append_query_errors(vec![DbErr::Custom("E4".to_string())])
        .into_connection();
    assert!(matches!(server_service::join_server(&db4, &tx, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));

    // Line 380: Find User Info Error
    let db5 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1234 }]])
        .append_query_results(vec![vec![] as Vec<server_ban::Model>])
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member }]])
        .append_query_errors(vec![DbErr::Custom("E5".to_string())])
        .into_connection();
    assert!(matches!(server_service::join_server(&db5, &tx, create_claims(user_id), srv_id, req.clone()).await, Err(AppError::InternalServerError(_))));
}

// 7. LEAVE SERVER ERRORS
#[tokio::test]
async fn test_leave_server_db_errors() {
    let srv_id = 66;
    let user_id = Uuid::new_v4();
    let (tx, _rx) = broadcast::channel(1);


    // Line 444: Find Server
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("E1".to_string())])
        .into_connection();
    assert!(matches!(server_service::leave_server(&db1, &tx, create_claims(user_id), srv_id).await, Err(AppError::InternalServerError(_))));

    // Line 452: Find Membership
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 }]])
        .append_query_errors(vec![DbErr::Custom("E2".to_string())])
        .into_connection();
    assert!(matches!(server_service::leave_server(&db2, &tx, create_claims(user_id), srv_id).await, Err(AppError::InternalServerError(_))));

    // Line 460: Delete Execute
    let db3 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model{ id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 }]])
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member }]])
        .append_exec_errors(vec![DbErr::Custom("E3".to_string())])
        .into_connection();
    assert!(matches!(server_service::leave_server(&db3, &tx, create_claims(user_id), srv_id).await, Err(AppError::InternalServerError(_))));
}

// 8. UPDATE MEMBER & CREATE CHANNEL MISSING ERRORS (Last few lines)
#[tokio::test]
async fn test_misc_update_channel_errors() {
    let (tx, _rx) = broadcast::channel(1);
    let srv_id = 55;
    let user_id = Uuid::new_v4();

    // update_member Line 518 (Requester check)
    let db1 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("E1".to_string())])
        .into_connection();
    let req = UpdateMemberRequest { new_role: "admin".to_string() };
    assert!(matches!(server_service::update_member(&db1, &tx, create_claims(user_id), srv_id, Uuid::new_v4(), req).await, Err(AppError::InternalServerError(_))));

    // create_channel Line 647 (Membership check)
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("E2".to_string())])
        .into_connection();
    let req_ch = CreateChannelRequest { name: "C".to_string(), description: "".to_string() };
    assert!(matches!(server_service::create_channel(&db2, &tx, create_claims(user_id), srv_id, req_ch).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_full_mapping_and_string_conversion_coverage() {
    // Ce test active les closures map() pour les membres (L205) et les channels (L188)
    // ainsi que la conversion "Member" (L611) qui n'étaient pas cover.
    let srv_id = 900;
    let user_id = Uuid::new_v4();
    let (tx, _rx) = broadcast::channel(1);

    // Get Server By ID : Full Coverage
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member }]])
        .append_query_results(vec![vec![server_model::Model { id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1 }]])
        .append_query_results(vec![vec![channel::Model { id: Uuid::new_v4(), server_id: srv_id, name: "C".to_string(), description: "D".to_string(), position: 0 }]])
        .append_query_results(vec![vec![(
             server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member },
             Some(user::Model { id: user_id, username: "U".to_string(), display_name: None, avatar_url: None, password_hash: "".to_string(), status: UserStatus::Online })
        )]])
        .into_connection();
    
    let res = server_service::get_server_by_id(&db, create_claims(user_id), srv_id).await;
    assert!(res.is_ok());
    assert!(res.unwrap().server.members.len() > 0);

    // Update Member : "Member" role coverage (Line 611)
    let db2 = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id: user_id, role: MemberRole::Owner }]])
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id: Uuid::new_v4(), role: MemberRole::Admin }]])
        .append_query_results(vec![vec![server_member::Model{ id: Uuid::new_v4(), server_id: srv_id, user_id: Uuid::new_v4(), role: MemberRole::Member }]])
        .append_query_results(vec![vec![user::Model { id: Uuid::new_v4(), username: "U".to_string(), display_name: None, avatar_url: None, password_hash: "x".to_string(), status: UserStatus::Online }]])
        .into_connection();
    
    let req = UpdateMemberRequest { new_role: "member".to_string() };
    let res2 = server_service::update_member(&db2, &tx, create_claims(user_id), srv_id, Uuid::new_v4(), req).await;
    assert!(res2.is_ok());
    assert_eq!(res2.unwrap().new_user.role, "Member");
}
