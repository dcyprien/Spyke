use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelItem {
    pub id: Uuid,
    pub server_id: i32,
    pub name: String,
    pub description: String,
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct GetChannelResponse {
    pub channel: ChannelItem
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub position: Option<i32>
}

#[derive(Debug, Serialize)]
pub struct UpdateChannelResponse {
    pub channel: ChannelItem
}

#[derive(Deserialize)]
pub struct TypingEvent {
    pub server_id: Uuid,
}