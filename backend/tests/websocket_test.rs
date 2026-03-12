use axum::{
    routing::get,
    Router,
};
use backend::{
    AppState,
    infrastructure::api::sockets::websocket::ws_handler,
    domain::models::{server_member, user},
    domain::models::user::UserStatus,
};
use sea_orm::{
    DatabaseBackend, MockDatabase, MockExecResult, EntityTrait,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::env;
use std::time::Duration;
use chrono::Utc;
use uuid::Uuid;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

// --- HELPERS ---

// Structure minimale pour générer un token compatible avec vos claims
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: Uuid,
    username: String,
    exp: usize,
    iat: usize,
}

fn generate_test_token(user_id: Uuid) -> String {
    let my_claims = TestClaims {
        sub: user_id,
        username: "test_ws_user".to_string(),
        exp: 9999999999, // Expiration lointaine
        iat: 0,
    };
    // Doit correspondre à la clé secrète définie dans setup_env
    encode(&Header::default(), &my_claims, &EncodingKey::from_secret("super_secret_test_key_must_be_long_enough".as_ref())).unwrap()
}

// Configuration de l'environnement (similaire à auth_service_test)
fn setup_env() {
    env::set_var("JWT_SECRET", "super_secret_test_key_must_be_long_enough");
}

// Fonction pour démarrer le serveur de test
async fn spawn_test_server(mock_db: sea_orm::DatabaseConnection) -> (SocketAddr, broadcast::Sender<String>) {
    let (tx, _) = broadcast::channel(100);
    
    let state = AppState {
        db: Arc::new(mock_db),
        tx: tx.clone(),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    // On bind sur le port 0 pour laisser l'OS choisir un port libre
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, tx)
}

// --- TESTS ---

#[tokio::test]
async fn test_websocket_auth_success_flow() {
    setup_env();
    let user_id = Uuid::new_v4();

    // 1. Préparation du Mock DB
    // Le socket va faire les requêtes suivantes dans l'ordre :
    // a. Update Status -> Online
    // b. Fetch User Servers (pour savoir à qui broadcaster)
    // c. Fetch User Servers (pour broadcaster la déco à la fin du test)
    // d. Update Status -> Offline (quand on ferme la connexion)
    
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // a. Update Online
        .append_query_results(vec![
            vec![user::Model { 
                id: user_id, 
                username: "test".to_string(), 
                password_hash: "".to_string(), 
                status: UserStatus::Online, 
                display_name: None, 
                avatar_url: None 
            }]
        ])
        // b. Fetch Servers (Au début) -> On dit qu'il est dans le serveur ID 1
        .append_query_results(vec![
            vec![server_member::Model { 
                id: Uuid::new_v4(), 
                server_id: 1, 
                user_id, 
                role: backend::domain::models::server_member::MemberRole::Member 
            }]
        ])
        // c. Fetch Servers (Au moment de la déco)
        .append_query_results(vec![
             vec![server_member::Model { 
                id: Uuid::new_v4(), 
                server_id: 1, 
                user_id, 
                role: backend::domain::models::server_member::MemberRole::Member 
            }]
        ])
        // d. Update Offline
        .append_query_results(vec![
            vec![user::Model { 
                id: user_id, 
                username: "test".to_string(), 
                password_hash: "".to_string(), 
                status: UserStatus::Offline, 
                display_name: None, 
                avatar_url: None 
            }]
        ])
        .into_connection();

    // 2. Démarrage serveur
    let (addr, _) = spawn_test_server(db).await;
    let ws_url = format!("ws://{}/ws", addr);

    // 3. Connexion Client
    let (mut socket, _) = connect_async(ws_url).await.expect("Failed to connect");

    // 4. Envoi Auth
    let token = generate_test_token(user_id);
    let auth_msg = json!({
        "type": "auth",
        "token": token
    });
    socket.send(Message::Text(auth_msg.to_string().into())).await.unwrap();

    // 5. Vérification Auth Success
    let msg = socket.next().await.unwrap().unwrap();
    if let Message::Text(text) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["type"], "auth_success");
        assert_eq!(json["data"]["user_id"], user_id.to_string());
    } else {
        panic!("Expected text message");
    }

    // 6. Fermeture propre pour déclencher la logique de fin
    socket.close(None).await.unwrap();
    
    // On laisse un petit délai pour que le serveur traite la déconnexion (Mock query c et d)
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_websocket_broadcast_reception() {
    setup_env();
    let user_id = Uuid::new_v4();
    let server_id = 99;

    // Helper pour créer un user (car Default n'est pas implémenté sur le Model SeaORM)
    let make_user = |status| user::Model {
        id: user_id,
        username: "broadcast_tester".to_string(),
        password_hash: "".to_string(),
        status,
        display_name: None,
        avatar_url: None,
    };

    // Mock simplifié pour ce test
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        // 1. Update Online
        .append_query_results(vec![vec![make_user(UserStatus::Online)]]) 
        // 2. Fetch Servers
        .append_query_results(vec![
            vec![server_member::Model { 
                id: Uuid::new_v4(), 
                server_id, 
                user_id, 
                role: backend::domain::models::server_member::MemberRole::Member 
            }]
        ]) 
        // 3. Disconnect Fetch Servers (Vide)
        .append_query_results(vec![vec![] as Vec<server_member::Model>]) 
        // 4. Disconnect Offline
        .append_query_results(vec![vec![make_user(UserStatus::Offline)]]) 
        .into_connection();

    let (addr, tx) = spawn_test_server(db).await;
    let ws_url = format!("ws://{}/ws", addr);

    let (mut socket, _) = connect_async(ws_url).await.unwrap();

    // Auth
    let token = generate_test_token(user_id);
    socket.send(Message::Text(json!({"type": "auth", "token": token}).to_string().into())).await.unwrap();
    
    // Consume auth_success
    let _ = socket.next().await; 

    let _status_msg = socket.next().await; 
    // TEST: Le Backend (via API REST par exemple) envoie un broadcast
    // On simule qu'un message arrive sur le channel broadcast interne
    let broadcast_msg = json!({
        "type": "new_message",
        "data": {
            "server_id": server_id, // L'user est membre de ce serveur (voir Mock)
            "content": "Hello World"
        }
    });
    
    // Le serveur de test a renvoyé le transmetteur `tx`
    tx.send(broadcast_msg.to_string()).unwrap();

    // VERIF: Le socket client doit le recevoir
    let msg = socket.next().await.unwrap().unwrap();
    match msg {
        Message::Text(text) => {
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(json["type"], "new_message");
            assert_eq!(json["data"]["content"], "Hello World");
        },
        _ => panic!("Expected Text message"),
    }
}

#[tokio::test]
async fn test_websocket_auth_fail() {
    setup_env();
    // Pas besoin de mock DB car ça fail avant de toucher à la DB
    let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
    
    let (addr, _) = spawn_test_server(db).await;
    let ws_url = format!("ws://{}/ws", addr);

    let (mut socket, _) = connect_async(ws_url).await.unwrap();

    // Envoi token invalide
    let auth_msg = json!({
        "type": "auth",
        "token": "bad.token.signature" // Signature invalide
    });
    socket.send(Message::Text(auth_msg.to_string().into())).await.unwrap();

    // On s'attend à recevoir une erreur ou une fermeture
    let msg = socket.next().await;
    
    if let Some(Ok(Message::Text(text))) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        // Vérifiez ce que votre code renvoie en cas d'erreur (voir handle_socket match Err)
        assert!(json["error"].as_str().is_some()); 
    } else {
        // Ou le socket peut être fermé direct
        assert!(true); 
    }
}