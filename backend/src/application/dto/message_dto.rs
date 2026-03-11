use sea_orm::prelude::DateTimeUtc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessageRequest {
    pub content: String,
    pub server_id: Option<i32>,
    pub direct_message: Option<Uuid>
}

#[derive(Debug, Serialize, Clone)]
pub struct ReactionItem {
    pub emoji: String,
    pub user_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageItem {
    pub id: Uuid,
    pub content: String,
    pub user_id: Uuid,
    pub author: String,
    pub channel_id: Option<Uuid>,
    pub direct_message_id: Option<Uuid>,
    pub server_id: Option<i32>,
    pub created_at: DateTimeUtc,
    pub reactions: Vec<ReactionItem>,
}

#[derive(Debug, Serialize)]
pub struct GetMessagesResponse {
    pub message_list: Vec<MessageItem>
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessageRequest {
    pub new_content: String
}

#[derive(Debug, Serialize)]
pub struct UpdateMessageResponse {
    pub new_message: MessageItem
}

#[derive(Debug, Deserialize)]
pub struct ToggleReactionRequest {
    pub emoji: String,
}