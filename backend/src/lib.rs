pub mod infrastructure;
pub mod application;
pub mod domain;

use std::sync::Arc;
use sea_orm::DatabaseConnection;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub tx: broadcast::Sender<String>
}