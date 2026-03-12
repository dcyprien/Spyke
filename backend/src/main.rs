use backend::{infrastructure, AppState};
use axum::{
    routing::{get, post, put, delete},
    Router,
    middleware,
};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use axum::http::{Method, HeaderValue};
use sea_orm::{Database, DatabaseConnection};
use std::env;
use std::net::SocketAddr;
use migration::{Migrator, MigratorTrait};
use infrastructure::api::sockets::websocket;
use infrastructure::api::handlers::{auth, server, channel, message};
use infrastructure::api::middlewares::middleware::auth_middleware;


#[tokio::main]
async fn main() {
    // Env variables
    dotenvy::dotenv().ok();

    // Logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not found");
    
    println!("Connecting to DB");
    let db: DatabaseConnection = match Database::connect(&db_url).await {
        Ok(conn) => {
            println!("Connected to DB");
            conn
        },
        Err(e) => {
            println!("Error connecting to DB: {}", e);
            std::process::exit(1);
        }
    };
    
    Migrator::up(&db, None).await.expect("Impossible de migrer la DB");

    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT doit être un nombre");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");
    
    let (tx, _rx) = broadcast::channel(100);

    let state = AppState { 
        db: std::sync::Arc::new(db),
        tx: tx
    };

    let protected_routes = Router::new()
    .route("/me", get(auth::me))
    .route("/auth/logout", post(auth::logout))
    .route("/auth/status", put(auth::update_status))
    .route("/servers", post(server::create_server)
                        .get(server::get_servers))
    .route("/servers/{id}", get(server::get_server_by_id)
                            .put(server::update_server)
                            .delete(server::delete_server))
    .route("/servers/{id}/join", post(server::join_server))
    .route("/servers/{id}/leave", delete(server::leave_server))
    .route("/servers/{id}/members", get(server::get_servermembers))
    .route("/servers/{server_id}/members/{userid}", put(server::update_member))
    .route("/servers/{server_id}/kick/{userid}", delete(server::kick_user))
    .route("/servers/{server_id}/ban/{userid}", post(server::ban_user))
    .route("/servers/{id}/channels", post(server::create_channel)
                                    .get(server::get_channels))
    .route("/channels/{id}", get(channel::get_channel_by_id)
                            .put(channel::update_channel)
                            .delete(channel::delete_channel))
    .route("/channels/{id}/messages", post(message::send_message)
                                    .get(message::get_messages))
    .route("/dm/{id}/messages", post(message::send_dm)
                                .get(message::get_direct_messages))
    .route("/dm", get(message::get_dm_list))
    .route("/messages/{id}", delete(message::delete_message)
                             .put(message::update_message))
    .route("/messages/{id}/reactions", put(message::toggle_reaction))
    .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let cors = CorsLayer::new()
    .allow_origin([
        HeaderValue::from_static("http://localhost:3001"),
    ])
    .allow_methods([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ])
    .allow_headers([
        axum::http::header::CONTENT_TYPE,
        axum::http::header::AUTHORIZATION,
    ])
    .allow_credentials(true);


    let app = Router::new()
        .route("/auth/signup", post(auth::signup))
        .route("/auth/login", post(auth::login))
        .route("/ws",get(websocket::ws_handler))
        .merge(protected_routes)
        .layer(cors)
        .with_state(state);

    
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}