use sea_orm::prelude::DateTimeUtc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessageRequest {
    pub content: String,
    pub server_id: Option<i32>,
    pub target_id: Option<Uuid>
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
    pub created_at: DateTimeUtc
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

#[derive(Debug, Serialize)]
pub struct DmItem {
    pub id: Uuid,
    pub user1 : Uuid,
    pub user2 : Uuid,
    pub user1_username: String,
    pub user2_username: String
}

#[derive(Debug, Serialize)]
pub struct GetDMSresponse {
    pub dm_list : Vec<DmItem>
}