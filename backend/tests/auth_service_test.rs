use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHasher};
use tokio::sync::broadcast;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseBackend, DbErr, EntityTrait, MockDatabase,
    MockExecResult, Transaction
};

use backend::application::services::auth_service;
use backend::application::dto::auth_dto::{SignupRequest, LoginRequest, RefreshRequest};
use backend::application::dto::token_dto::Claims;
use backend::application::dto::apperror::AppError; // Assurez-vous que l'import est correct
use backend::domain::models::{user, refresh_token, server_member, server_model, channel};
use backend::domain::models::user::UserStatus;
use std::env;
use std::sync::Mutex;
use chrono::{Duration, Utc};
use uuid::Uuid;

// Mutex pour gérer la variable d'env JWT_SECRET qui est globale
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn setup_env() {
    env::set_var("JWT_SECRET", "super_secret_test_key_must_be_long_enough");
}

// Helper pour générer un hash valide le temps du test
fn generate_hash(pwd: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pwd.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

// --- SUITE 1 : REGISTER ---

#[tokio::test]
async fn test_register_success() {
    let user_id_mock = Uuid::new_v4();
    let username = "new_user".to_string();

    // Mock DB : On simule que tout se passe bien.
    // L'ID mis ici dans le mock ne sera PAS celui retourné par le service (voir explication ci-dessus),
    // mais c'est nécessaire pour que SeaORM ne plante pas.
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![
            vec![] as Vec<user::Model>, // 1. Find: Aucun résultat (User n'existe pas encore)
        ])
        .append_query_results(vec![
            vec![user::Model {
                id: user_id_mock, 
                username: username.clone(),
                password_hash: "hash".to_string(),
                status: UserStatus::Offline,
                display_name: None,
                avatar_url: None,
            }]
        ])
        .into_connection();

    let req = SignupRequest {
        username: username.clone(),
        password: "password123".to_string(),
    };

    let res = auth_service::register_user(&db, req).await;

    assert!(res.is_ok());
    let data = res.unwrap();
    
    // 1. On vérifie le username
    assert_eq!(data.username, username);
    
    // 2. CORRECTION : On ne compare PAS data.id == user_id_mock.
    // On vérifie juste que l'ID retourné n'est pas le Uuid::nil (0000...)
    assert_ne!(data.id, Uuid::nil(), "L'ID retourné ne doit pas être nul");
}

#[tokio::test]
async fn test_register_password_too_short() {
    let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
    let req = SignupRequest {
        username: "user".to_string(),
        password: "short".to_string(), // < 8
    };

    let res = auth_service::register_user(&db, req).await;
    assert!(matches!(res, Err(AppError::BadRequest(_))));
    if let Err(AppError::BadRequest(msg)) = res {
        assert_eq!(msg, "Password too short");
    }
}

#[tokio::test]
async fn test_register_username_taken() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![
            // Find renvoie un utilisateur existant
            vec![user::Model {
                id: Uuid::new_v4(),
                username: "taken".to_string(),
                password_hash: "hash".to_string(),
                status: UserStatus::Offline,
                display_name: None,
                avatar_url: None,
            }],
        ])
        .into_connection();

    let req = SignupRequest {
        username: "taken".to_string(),
        password: "password123".to_string(),
    };

    let res = auth_service::register_user(&db, req).await;
    assert!(matches!(res, Err(AppError::BadRequest(_))));
    if let Err(AppError::BadRequest(msg)) = res {
        assert_eq!(msg, "Username already in use");
    }
}

#[tokio::test]
async fn test_register_db_error_on_insert() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<user::Model>]) // Find: OK (vide)
        .append_query_errors(vec![DbErr::Custom("DB Crash".to_string())]) // User Insert: Crash
        .into_connection();

    let req = SignupRequest {
        username: "fail".to_string(),
        password: "password123".to_string(),
    };

    let res = auth_service::register_user(&db, req).await;
    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

// --- SUITE 2 : LOGIN ---

#[tokio::test]
async fn test_login_success_create_new_token() {
    setup_env();

    let user_id = Uuid::new_v4();
    let pwd = "mypassword";
    let hash = generate_hash(pwd);

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find User
        .append_query_results(vec![
            vec![user::Model { id: user_id, username: "ok".to_string(), password_hash: hash, status: UserStatus::Offline, display_name: None, avatar_url: None }]
        ])
        // 2. [BROADCAST] Update Status -> Online (Retourne le user)
        .append_query_results(vec![
            vec![user::Model { id: user_id, username: "ok".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]
        ])
        // 3. [BROADCAST] Find Memberships
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // 4. Find Existing Token (Vide)
        .append_query_results(vec![vec![] as Vec<refresh_token::Model>])
        // 5. Insert New Token
        .append_query_results(vec![
            vec![refresh_token::Model { id: Uuid::new_v4(), user_id, token: "new".to_string(), expires_at: (Utc::now() + Duration::hours(48)).into(), created_at: Utc::now().into() }]
        ])
        .into_connection();

    let (tx, _) = broadcast::channel(1);
    let req = LoginRequest { username: "ok".to_string(), password: pwd.to_string() };
    let res = auth_service::login_user(&db, &tx, req).await;

    assert!(res.is_ok());
    assert!(!res.unwrap().access_token.is_empty());
}

#[tokio::test]
async fn test_login_user_not_found() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<user::Model>]) // User vide
        .into_connection();

    let req = LoginRequest { username: "ghost".to_string(), password: "pwd".to_string() };
     let (tx, _) = broadcast::channel(1); // AJOUT DU CHANNEL
    let res = auth_service::login_user(&db, &tx, req).await;

    assert!(matches!(res, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_login_invalid_password() {
    // On génère un hash pour "vrai_mdp"
    let hash = generate_hash("vrai_mdp");

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![
            vec![user::Model {
                id: Uuid::new_v4(),
                username: "u".to_string(),
                password_hash: hash,
                status: UserStatus::Offline,
                display_name: None,
                avatar_url: None,
            }],
        ])
        // Pas d'autres mock car ça doit fail avant la DB
        .into_connection();

    // On tente de se log avec "faux_mdp"
    let req = LoginRequest { username: "u".to_string(), password: "faux_mdp".to_string() };
     let (tx, _) = broadcast::channel(1); // AJOUT DU CHANNEL
    let res = auth_service::login_user(&db, &tx, req).await;

    assert!(matches!(res, Err(AppError::Unauthorized(_))));
}

#[tokio::test]
async fn test_login_existing_token_valid() {
    setup_env();
    let user_id = Uuid::new_v4();
    let hash = generate_hash("pwd");

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. User
        .append_query_results(vec![
            vec![user::Model { id: user_id, username: "u".to_string(), password_hash: hash, status: UserStatus::Offline, display_name: None, avatar_url: None }],
        ])
        // 2. Update Status Online
        .append_query_results(vec![
            vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]
        ])
        // 3. Memberships
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // 4. Token Existant VALIDE
        .append_query_results(vec![
            vec![refresh_token::Model { id: Uuid::new_v4(), user_id, token: "existing_token".to_string(), expires_at: (Utc::now() + Duration::hours(1)).into(), created_at: Utc::now().into() }]
        ])
        .into_connection();

    let (tx, _) = broadcast::channel(1);
    let req = LoginRequest { username: "u".to_string(), password: "pwd".to_string() };
    let res = auth_service::login_user(&db, &tx, req).await;

    assert!(res.is_ok());
    assert_eq!(res.unwrap().access_token, "existing_token");
}

#[tokio::test]
async fn test_login_existing_token_expired() {
    setup_env();
    let user_id = Uuid::new_v4();
    let hash = generate_hash("pwd");

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. User
        .append_query_results(vec![
            vec![user::Model { id: user_id, username: "u".to_string(), password_hash: hash, status: UserStatus::Offline, display_name: None, avatar_url: None }],
        ])
        // 2. Status Online
        .append_query_results(vec![
             vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]
        ])
        // 3. Memberships
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // 4. Token Existant EXPIRÉ
        .append_query_results(vec![
            vec![refresh_token::Model { id: Uuid::new_v4(), user_id, token: "exp".to_string(), expires_at: (Utc::now() - Duration::hours(1)).into(), created_at: Utc::now().into() }]
        ])
        // 5. Delete old
        .append_exec_results(vec![MockExecResult { last_insert_id: 0, rows_affected: 1 }])
        // 6. Insert new
        .append_query_results(vec![
            vec![refresh_token::Model { id: Uuid::new_v4(), user_id, token: "new".to_string(), expires_at: (Utc::now() + Duration::hours(48)).into(), created_at: Utc::now().into() }]
        ])
        .into_connection();

    let (tx, _) = broadcast::channel(1);
    let req = LoginRequest { username: "u".to_string(), password: "pwd".to_string() };
    let res = auth_service::login_user(&db, &tx, req).await;

    assert!(res.is_ok());
    assert_ne!(res.unwrap().access_token, "exp");
}

// --- SUITE 3 : LOGOUT ---

#[tokio::test]
async fn test_logout_success() {
    let user_id = Uuid::new_v4();
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Update Status -> Offline (Retourne user modifié)
        .append_query_results(vec![
             vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Offline, display_name: None, avatar_url: None }]
        ])
        // 2. Fetch Memberships pour broadcast
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        // 3. Delete Tokens
        .append_exec_results(vec![
            MockExecResult { last_insert_id: 0, rows_affected: 5 } 
        ])
        .into_connection();

    let claims = Claims { sub: user_id, username: "u".to_string(), exp: 9999999999, iat: 0 };
    let (tx, _) = broadcast::channel(1);
    let res = auth_service::logout_user(&db, &tx, claims).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_logout_db_error() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_exec_errors(vec![DbErr::Custom("DB Fail".to_string())])
        .into_connection();

    let claims = Claims { sub: Uuid::new_v4(), username: "u".to_string(), exp: 0, iat: 0 };
     let (tx, _) = broadcast::channel(1); // AJOUT DU CHANNEL
    let res = auth_service::logout_user(&db, &tx, claims).await;

    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

// --- SUITE 4 : REFRESH TOKEN ---

#[tokio::test]
async fn test_refresh_success() {
    setup_env();
    let user_id = Uuid::new_v4();

    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![
            vec![refresh_token::Model { id: Uuid::new_v4(), user_id, token: "rt".to_string(), expires_at: (Utc::now() + Duration::hours(1)).into(), created_at: Utc::now().into() }]
        ])
        .append_query_results(vec![
            vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "x".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]
        ])
        .into_connection();

    let req = RefreshRequest { refresh_token: "rt".to_string() };
    let res = auth_service::refresh_access_token(&db, req).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_refresh_token_not_found() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<refresh_token::Model>])
        .into_connection();

    let req = RefreshRequest { refresh_token: "invalid".to_string() };
    let res = auth_service::refresh_access_token(&db, req).await;

    assert!(matches!(res, Err(AppError::Unauthorized(_))));
}

#[tokio::test]
async fn test_refresh_token_expired() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![
            vec![refresh_token::Model {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                token: "expired".to_string(),
                expires_at: (Utc::now() - Duration::hours(1)).into(), // Expiré
                created_at: Utc::now().into(),
            }]
        ])
        .into_connection();

    let req = RefreshRequest { refresh_token: "expired".to_string() };
    let res = auth_service::refresh_access_token(&db, req).await;

    assert!(matches!(res, Err(AppError::Unauthorized(_))));
    if let Err(AppError::Unauthorized(msg)) = res {
        assert_eq!(msg, "Refresh token expired");
    }
}

// --- SUITE 5 : ME (Complex Joins) ---

#[tokio::test]
async fn test_me_success_complex_structure() {
    let user_id = Uuid::new_v4();
    let server_id = 100;
    
    // Simulation complexe de SeaORM Mock pour les requêtes en cascade
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Find User by ID
        .append_query_results(vec![
            vec![user::Model {
                id: user_id,
                username: "me_user".to_string(),
                password_hash: "h".to_string(),
                status: UserStatus::Online,
                display_name: Some("Me Display".to_string()),
                avatar_url: None,
            }]
        ])
        // 2. Find Members (UserID) JOIN Server.
        // SeaORM Mock retourne des tuples pour les joins (Model, Some(RelatedModel))
        .append_query_results(vec![
            vec![(
                server_member::Model { 
                    id: Uuid::new_v4(), 
                    server_id: server_id, 
                    user_id: user_id, 
                    role: backend::domain::models::server_member::MemberRole::Owner 
                },
                Some(server_model::Model {
                    id: server_id,
                    name: "My Server".to_string(),
                    description: "Desc".to_string(),
                    icon_url: None,
                    invitcode: 12345,
                    owner_id: user_id,
                })
            )]
        ])
        // 3. Find Channels (IN [100])
        .append_query_results(vec![
            vec![channel::Model {
                id: Uuid::new_v4(),
                server_id: server_id,
                name: "General".to_string(),
                description: "Main chat".to_string(),
                position: 0,
            }]
        ])
        // 4. Find ALL Members for server 100 JOIN User
        .append_query_results(vec![
            // Membre 1 : Moi
             vec![(
                server_member::Model { id: Uuid::new_v4(), server_id, user_id, role: backend::domain::models::server_member::MemberRole::Owner },
                Some(user::Model { id: user_id, username: "me_user".to_string(), password_hash: "h".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None })
            )]
        ])
        .into_connection();

    let claims = Claims { sub: user_id, username: "u".to_string(), exp: 0, iat: 0 };
    let res = auth_service::me(&db, claims).await;

    assert!(res.is_ok());
    let me = res.unwrap();
    
    assert_eq!(me.id, user_id);
    assert_eq!(me.servers.len(), 1);
    
    let serv = &me.servers[0];
    assert_eq!(serv.name, "My Server");
    assert_eq!(serv.channels.len(), 1);
    assert_eq!(serv.channels[0].name, "General");
    assert_eq!(serv.members.len(), 1);
    assert_eq!(serv.members[0].username, "me_user");
}

#[tokio::test]
async fn test_me_user_not_found() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![] as Vec<user::Model>])
        .into_connection();

    let claims = Claims { sub: Uuid::new_v4(), username: "ghost".to_string(), exp: 0, iat: 0 };
    let res = auth_service::me(&db, claims).await;

    assert!(matches!(res, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_me_db_crash() {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("DB Boom".to_string())])
        .into_connection();

    let claims = Claims { sub: Uuid::new_v4(), username: "u".to_string(), exp: 0, iat: 0 };
    let res = auth_service::me(&db, claims).await;

    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_register_find_user_db_error() {
    // Couvre Line 25: Erreur lors de la vérification initiale si le user existe
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Find User Connection Fail".to_string())])
        .into_connection();

    let req = SignupRequest { username: "u".to_string(), password: "password123".to_string() };
    let res = auth_service::register_user(&db, req).await;
    
    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_login_find_user_db_error() {
    // Couvre Line 64: Erreur lors de la recherche du user pour le login
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Login Find User Fail".to_string())])
        .into_connection();

    let req = LoginRequest { username: "u".to_string(), password: "p".to_string() };
         let (tx, _) = broadcast::channel(1); // AJOUT DU CHANNEL
    assert!(matches!(auth_service::login_user(&db, &tx,  req).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_login_corrupt_password_hash() {
    // Couvre Line 68: Le hash en base n'est pas un hash Argon2 valide
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![
            vec![user::Model {
                id: Uuid::new_v4(),
                username: "u".to_string(),
                password_hash: "NOT_A_VALID_HASH".to_string(), // <--- Ici
                status: UserStatus::Offline,
                display_name: None,
                avatar_url: None,
            }]
        ])
        .into_connection();

    let req = LoginRequest { username: "u".to_string(), password: "p".to_string() };
     let (tx, _) = broadcast::channel(1); // AJOUT DU CHANNEL
    let res = auth_service::login_user(&db,&tx ,req).await;
    
    // Doit retourner une InternalServerError car c'est une corruption de données, pas un Unauthorized
    assert!(matches!(res, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_login_find_refresh_token_db_error() {
    // Couvre Line 78: User trouvé, Password OK, mais Crash en cherchant le token
    let hash = generate_hash("pwd");
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![user::Model { id: Uuid::new_v4(), username: "u".to_string(), password_hash: hash, status: UserStatus::Offline, display_name: None, avatar_url: None }]])
        .append_query_errors(vec![DbErr::Custom("Find Token Fail".to_string())])
        .into_connection();

    let req = LoginRequest { username: "u".to_string(), password: "pwd".to_string() };
     let (tx, _) = broadcast::channel(1); // AJOUT DU CHANNEL
    assert!(matches!(auth_service::login_user(&db, &tx, req).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_login_delete_expired_token_exec_error() {
    // Couvre Line 88: Token expiré trouvé -> Tentative de suppression -> Crash DB
    let hash = generate_hash("pwd");
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. User
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: hash, status: UserStatus::Offline, display_name: None, avatar_url: None }]])
        // 2. Token Expiré
        .append_query_results(vec![vec![refresh_token::Model {
            id: Uuid::new_v4(), user_id, token: "exp".to_string(), 
            expires_at: (Utc::now() - Duration::hours(1)).into(), created_at: Utc::now().into()
        }]])
        // 3. Delete Error
        .append_exec_errors(vec![DbErr::Custom("Delete Token Fail".to_string())])
        .into_connection();

    let req = LoginRequest { username: "u".to_string(), password: "pwd".to_string() };
         let (tx, _) = broadcast::channel(1); // AJOUT DU CHANNEL
    assert!(matches!(auth_service::login_user(&db, &tx,  req).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_login_insert_new_token_error() {
    setup_env();
    let hash = generate_hash("pwd");
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: hash, status: UserStatus::Offline, display_name: None, avatar_url: None }]])
        .append_query_results(vec![vec![user::Model { id: user_id, username: "u".to_string(), password_hash: "".to_string(), status: UserStatus::Online, display_name: None, avatar_url: None }]])
        .append_query_results(vec![vec![] as Vec<server_member::Model>])
        .append_query_results(vec![vec![] as Vec<refresh_token::Model>])
        .append_query_errors(vec![DbErr::Custom("Fail".to_string())])
        .into_connection();

    let (tx, _) = broadcast::channel(1);
    let req = LoginRequest { username: "u".to_string(), password: "pwd".to_string() };
    assert!(matches!(auth_service::login_user(&db, &tx, req).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_refresh_token_find_error() {
    // Couvre Line 132
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_errors(vec![DbErr::Custom("Find RT Fail".to_string())])
        .into_connection();

    let req = RefreshRequest { refresh_token: "rt".to_string() };
    assert!(matches!(auth_service::refresh_access_token(&db, req).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_refresh_user_fetch_error() {
    // Couvre Line 143: Token valide found, mais User fetch fail
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![refresh_token::Model {
            id: Uuid::new_v4(), user_id: Uuid::new_v4(), token: "rt".to_string(),
            expires_at: (Utc::now() + Duration::hours(1)).into(), created_at: Utc::now().into()
        }]])
        .append_query_errors(vec![DbErr::Custom("Find User Fail".to_string())])
        .into_connection();

    let req = RefreshRequest { refresh_token: "rt".to_string() };
    assert!(matches!(auth_service::refresh_access_token(&db, req).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_me_fetch_servers_error() {
    // Couvre Line 166: User trouvé, mais erreur join servers
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![user::Model {
            id: Uuid::new_v4(), username: "u".to_string(), password_hash: "x".to_string(),
            status: UserStatus::Online, display_name: None, avatar_url: None
        }]])
        .append_query_errors(vec![DbErr::Custom("Fetch Servers Fail".to_string())])
        .into_connection();

    let claims = Claims { sub: Uuid::new_v4(), username: "u".to_string(), exp: 0, iat: 0 };
    assert!(matches!(auth_service::me(&db, claims).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_me_fetch_channels_error() {
    // Couvre Line 179: Servers trouvés, mais erreur channels
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![user::Model {
            id: user_id, username: "u".to_string(), password_hash: "x".to_string(),
            status: UserStatus::Online, display_name: None, avatar_url: None
        }]])
        .append_query_results(vec![vec![(
            server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id, role: backend::domain::models::server_member::MemberRole::Owner },
            Some(server_model::Model { id: 1, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: user_id, invitcode: 1 })
        )]])
        .append_query_errors(vec![DbErr::Custom("Fetch Channels Fail".to_string())])
        .into_connection();

    let claims = Claims { sub: user_id, username: "u".to_string(), exp: 0, iat: 0 };
    assert!(matches!(auth_service::me(&db, claims).await, Err(AppError::InternalServerError(_))));
}

#[tokio::test]
async fn test_me_fetch_members_error() {
    // Couvre Line 187: Channels OK, mais erreur fetch all members
    let user_id = Uuid::new_v4();
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results(vec![vec![user::Model {
            id: user_id, username: "u".to_string(), password_hash: "x".to_string(),
            status: UserStatus::Online, display_name: None, avatar_url: None
        }]])
        .append_query_results(vec![vec![(
            server_member::Model { id: Uuid::new_v4(), server_id: 1, user_id, role: backend::domain::models::server_member::MemberRole::Owner },
            Some(server_model::Model { id: 1, name: "S".to_string(), description: "D".to_string(), icon_url: None, owner_id: user_id, invitcode: 1 })
        )]])
        .append_query_results(vec![vec![] as Vec<channel::Model>]) // 0 channels OK
        .append_query_errors(vec![DbErr::Custom("Fetch Members Fail".to_string())])
        .into_connection();

    let claims = Claims { sub: user_id, username: "u".to_string(), exp: 0, iat: 0 };
    assert!(matches!(auth_service::me(&db, claims).await, Err(AppError::InternalServerError(_))));
}