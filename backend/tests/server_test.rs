use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{delete, get, post, put},
    Router,
};
use backend::{
    application::dto::{server_dto::CreateServerRequest, token_dto::Claims},
    domain::models::{server_model, server_member, channel, user},
    domain::models::server_member::MemberRole,
    AppState,
    infrastructure::api::handlers::server,
};
use sea_orm::{DatabaseBackend, DbErr, MockDatabase};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt; // pour oneshot
use uuid::Uuid;
use chrono::Utc;
use tokio::sync::broadcast;

// Configuration du routeur de test avec Mock DB et Broadcast Channel
fn setup_router(db: sea_orm::DatabaseConnection) -> Router {
    let (tx, _rx) = broadcast::channel(100);

    // CORRECTION : Si votre AppState nécessite Arc, ajoutez-le.
    // Cependant, l'erreur "expected struct `Arc` found enum" suggère que le code appelant
    // ou la structure elle-même attend un Arc.
    
    // CAS 1 : Si votre AppState est : struct AppState { db: DatabaseConnection, ... }
    let state = AppState {
        db: db.into(), 
        tx: tx,
    };
    
    // Axum avec .with_state attend souvent un State qui implémente Clone.
    // DatabaseConnection est Clone (c'est un handle). Broadcast Sender est Clone.
    // Donc ça devrait passer.

    // SI L'ERREUR PERSISTE, c'est peut-être la méthode `create_server` du handler qui
    // demande `State(state): State<Arc<AppState>>` ?
    
    Router::new()
        .route("/servers", post(server::create_server).get(server::get_servers))
        .route("/servers/{id}", get(server::get_server_by_id).put(server::update_server).delete(server::delete_server))
        .route("/servers/{id}/join", post(server::join_server))
        .route("/servers/{id}/leave", delete(server::leave_server))
        .route("/servers/{id}/members", get(server::get_servermembers))
        .route("/servers/{server_id}/members/{user_id}", put(server::update_member))
        .route("/servers/{id}/channels", post(server::create_channel).get(server::get_channels))
        .with_state(state) 
}

fn create_test_claims(user_id: Uuid) -> Claims {
    Claims {
        sub: user_id,
        username: "tester".to_string(),
        exp: Utc::now().timestamp() as usize + 3600,
        iat: Utc::now().timestamp() as usize,
    }
}

// --- TEST CREATE SERVER ---

#[tokio::test]
async fn test_create_server_created() {
    let user_id = Uuid::new_v4();
    
    // Le service fait : 
    // 1. Check nom unique (Empty result)
    // 2. Insert Server
    // 3. Insert Membership Owner
    // 4. Insert Default Channel
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<server_model::Model>]) 
        .append_query_results(vec![vec![server_model::Model {
            id: 1, name: "New Srv".to_string(), description: "Desc".to_string(), icon_url: None, owner_id: user_id, invitcode: 123
        }]])
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: 1, user_id: user_id, role: MemberRole::Owner,
        }]])
        .append_query_results(vec![vec![channel::Model {
            id: Uuid::new_v4(), server_id: 1, name: "general".to_string(), description: "".to_string(), position: 0
        }]])
        .into_connection();

    let app = setup_router(db);
    let claims = create_test_claims(user_id);
    let payload = json!({ "name": "New Srv", "description": "Desc" });

    let req = Request::builder()
        .method("POST")
        .uri("/servers")
        .header("content-type", "application/json")
        .extension(claims)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
}

// --- TEST GET SERVERS ---

#[tokio::test]
async fn test_get_servers_ok() {
    let user_id = Uuid::new_v4();
    // Le service fait des jointures complexes, on mocke un résultat vide pour simplifier le test Handler
    // L'important est que le handler reçoive Ok(_) du service
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<(server_member::Model, Option<server_model::Model>)>])
        // Channels
        .append_query_results(vec![vec![] as Vec<channel::Model>])
        // Members
        .append_query_results(vec![vec![] as Vec<(server_member::Model, Option<user::Model>)>])
        .into_connection();

    let app = setup_router(db);
    let claims = create_test_claims(user_id);

    let req = Request::builder()
        .uri("/servers")
        .extension(claims)
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- TEST GET SERVER BY ID ---

#[tokio::test]
async fn test_get_server_by_id_found() {
    let user_id = Uuid::new_v4();
    let srv_id = 99;

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // Check Membership
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        // Get Server
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1
        }]])
        // Get Channels
        .append_query_results(vec![vec![] as Vec<channel::Model>])
        // Get Members
        .append_query_results(vec![vec![] as Vec<(server_member::Model, Option<user::Model>)>])
        .into_connection();

    let app = setup_router(db);
    let claims = create_test_claims(user_id);

    let req = Request::builder()
        .uri(&format!("/servers/{}", srv_id))
        .extension(claims)
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_server_by_id_forbidden() {
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // Check Membership -> Vide = Pas membre
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .into_connection();

    let app = setup_router(db);
    let req = Request::builder()
        .uri("/servers/1")
        .extension(create_test_claims(user_id))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND); // Ou FORBIDDEN selon votre implémentation du service
}

// --- TEST JOIN SERVER ---

#[tokio::test]
async fn test_join_server_success() {
    let user_id = Uuid::new_v4();
    let srv_id = 10;
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Get Server
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "JoinMe".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1234
        }]])
        // 2. Check Member -> Vide
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // 3. Insert Member
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        // 4. Get User Info (pour Broadcast)
        .append_query_results(vec![vec![user::Model {
            id: user_id, username: "U".to_string(), display_name: None, avatar_url: None, status: user::UserStatus::Online, password_hash: "pass".to_string()
        }]])
        // 5. Channels
        .append_query_results(vec![vec![] as Vec<channel::Model>])
        // 6. Members List
        .append_query_results(vec![vec![] as Vec<(server_member::Model, Option<user::Model>)>])
        .into_connection();

    let app = setup_router(db);
    let payload = json!({ "invitcode": 1234 });

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/servers/{}/join", srv_id))
        .header("content-type", "application/json")
        .extension(create_test_claims(user_id))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- TEST UPDATE MEMBER (ROLE) ---

#[tokio::test]
async fn test_update_member_role() {
    let srv_id = 5;
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Requester check (Owner)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id: owner_id, role: MemberRole::Owner
        }]])
        // 2. Target check
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id: target_id, role: MemberRole::Member
        }]])
        // 3. Update execution
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id: target_id, role: MemberRole::Admin
        }]])
        // 4. User Info (Broadcast)
        .append_query_results(vec![vec![user::Model {
            id: target_id, username: "T".to_string(), display_name: None, avatar_url: None, status: user::UserStatus::Online, password_hash: "p".to_string()
        }]])
        .into_connection();

    let app = setup_router(db);
    let payload = json!({ "new_role": "admin" });

    let req = Request::builder()
        .method("PUT")
        .uri(&format!("/servers/{}/members/{}", srv_id, target_id))
        .header("content-type", "application/json")
        .extension(create_test_claims(owner_id))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- TEST CHANNELS ---

#[tokio::test]
async fn test_create_channel() {
    let srv_id = 8;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Perms
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Admin
        }]])
        // 2. Insert Channel
        .append_query_results(vec![vec![channel::Model {
            id: Uuid::new_v4(), server_id: srv_id, name: "chan".to_string(), description: "".to_string(), position: 0
        }]])
        .into_connection();

    let app = setup_router(db);
    let payload = json!({ "name": "chan", "description": "desc" });

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/servers/{}/channels", srv_id))
        .header("content-type", "application/json")
        .extension(create_test_claims(user_id))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK); // Ou OK selon le handler (votre handler renvoie OK)
}

// --- TEST UPDATE SERVER ---

#[tokio::test]
async fn test_update_server_success() {
    let srv_id = 20; // Type i32
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Membership (Le code service commence TOUJOURS par vérifier si on est membre)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner
        }]])
        // 2. Get Server (Ensuite il récupère le serveur pour créer l'ActiveModel)
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "Old Name".to_string(), description: "Old".to_string(), icon_url: None, owner_id: user_id, invitcode: 1
        }]])
        // 3. Update Server (Enfin il execute l'update. Il n'y a PAS de requête "Check Uniqueness" dans votre update_server)
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "Updated Name".to_string(), description: "Desc".to_string(), icon_url: None, owner_id: user_id, invitcode: 1
        }]])
        .into_connection();

    let app = setup_router(db);
    
    // On s'assure que le payload correspond exactement aux types attendus (i32 pour l'id)
    let payload = json!({ 
        "id": srv_id, 
        "name": "Updated Name".to_string(),
        "description": "Desc".to_string()
    });

    let req = Request::builder()
        .method("PUT")
        .uri(&format!("/servers/{}", srv_id))
        .header("content-type", "application/json")
        .extension(create_test_claims(user_id))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- TEST DELETE SERVER ---

#[tokio::test]
async fn test_delete_server_success() {
    let srv_id = 21;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Membership (Owner) - ON LE MET EN PREMIER
        // Si le service vérifie les droits avant de chercher le serveur, cet ordre est crucial.
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Owner
        }]])
        // 2. Get Server (Pour vérifier qu'il existe ou vérifier le owner_id interne)
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "To Delete".to_string(), description: "".to_string(), icon_url: None, owner_id: user_id, invitcode: 1
        }]])
        // 3. Delete execution
        .append_exec_results(vec![
             sea_orm::MockExecResult { last_insert_id: 0, rows_affected: 1 }
        ])
        .into_connection();

    let app = setup_router(db);
    
    let req = Request::builder()
        .method("DELETE")
        .uri(&format!("/servers/{}", srv_id))
        .extension(create_test_claims(user_id))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- TEST LEAVE SERVER ---

#[tokio::test]
async fn test_leave_server_success() {
    let srv_id = 22;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Get Server
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1
        }]])
        // 2. Check Membership (Member)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        // 3. Delete Membership
        .append_exec_results(vec![
             sea_orm::MockExecResult { last_insert_id: 0, rows_affected: 1 }
        ])
        .into_connection();

    let app = setup_router(db);

    let req = Request::builder()
        .method("DELETE")
        .uri(&format!("/servers/{}/leave", srv_id))
        .extension(create_test_claims(user_id))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- TEST GET SERVER MEMBERS ---

#[tokio::test]
async fn test_get_server_members_ok() {
    let srv_id = 30;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Membership
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        // 2. Get Members JOINS
        .append_query_results(vec![vec![
            (
                server_member::Model { id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member },
                Some(user::Model { id: user_id, username: "U".to_string(), display_name: None, avatar_url: None, status: user::UserStatus::Online, password_hash: "".to_string() })
            )
        ]])
        .into_connection();

    let app = setup_router(db);

    let req = Request::builder()
        .uri(&format!("/servers/{}/members", srv_id))
        .extension(create_test_claims(user_id))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- TEST GET CHANNELS ---

#[tokio::test]
async fn test_get_channels_ok() {
    let srv_id = 40;
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Check Membership
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id, role: MemberRole::Member
        }]])
        // 2. Get Channels
        .append_query_results(vec![vec![channel::Model {
             id: Uuid::new_v4(), server_id: srv_id, name: "general".to_string(), description: "".to_string(), position: 0
        }]])
        .into_connection();

    let app = setup_router(db);

    let req = Request::builder()
        .uri(&format!("/servers/{}/channels", srv_id))
        .extension(create_test_claims(user_id))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- ERROR CASES Handlers ---

#[tokio::test]
async fn test_create_server_failure() {
    // Simulation d'une erreur DB (ex: nom dupliqué)
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![server_model::Model { // Nom déjà pris, server trouvé
            id: 1, name: "Dup".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 0
        }]]) 
        .into_connection();

    let app = setup_router(db);
    let payload = json!({ "name": "Dup", "description": "D" });

    let req = Request::builder()
        .method("POST")
        .uri("/servers")
        .header("content-type", "application/json")
        .extension(create_test_claims(Uuid::new_v4()))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    // Votre handler mappe les erreurs services souvent vers BAD_REQUEST
    assert_eq!(res.status(), StatusCode::BAD_REQUEST); 
}

#[tokio::test]
async fn test_join_server_invalid_code() {
    let srv_id = 99;
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Server found (invitcode = 1234)
        .append_query_results(vec![vec![server_model::Model {
            id: srv_id, name: "S".to_string(), description: "".to_string(), icon_url: None, owner_id: Uuid::new_v4(), invitcode: 1234
        }]])
        // 2. Not member
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .into_connection();

    let app = setup_router(db);
    let payload = json!({ "invitcode": 0000 }); // Code invalide

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/servers/{}/join", srv_id))
        .header("content-type", "application/json")
        .extension(create_test_claims(Uuid::new_v4()))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_channel_no_permission() {
    let srv_id = 88;
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Membership: Member (Not admin -> Forbidden)
        .append_query_results(vec![vec![server_member::Model {
            id: Uuid::new_v4(), server_id: srv_id, user_id: Uuid::new_v4(), role: MemberRole::Member
        }]])
        .into_connection();

    let app = setup_router(db);
    let payload = json!({ "name": "c", "description": "" });

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/servers/{}/channels", srv_id))
        .header("content-type", "application/json")
        .extension(create_test_claims(Uuid::new_v4()))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    // Le handler error catch AppError::Forbidden et peut renvoyer BAD_REQUEST
    // Vérifiez si votre handler renvoie 400 ou 403. Ici je suppose 400 vu votre code précédent.
    assert_eq!(res.status(), StatusCode::BAD_REQUEST); 
}