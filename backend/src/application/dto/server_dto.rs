use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::dto::channel_dto::ChannelItem;

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateServerResponse {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub invitcode: i32,
    pub owner_id: Uuid,
    pub channels: Vec<ChannelItem>
}

#[derive(Debug, Serialize)]
pub struct ServerItem {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub owner_id: Uuid,
    pub admins: Vec<Uuid>, // AJOUT ICI
    pub invitcode: i32,
    pub channels: Vec<ChannelItem>,
    pub members: Vec<MemberItem>
}

#[derive(Debug, Serialize)]
pub struct GetServerResponse {
    pub server_list: Vec<ServerItem>
}

#[derive(Debug, Serialize)]
pub struct GetServerIdResponse {
    pub server: ServerItem
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpdateServerRequest {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>
}

#[derive(Debug, Serialize)]
pub struct UpdateServerResponse {
    pub id: i32,
    pub new_name: Option<String>,
    pub new_description: Option<String>,
    pub new_icon_url: Option<String>
}

#[derive(Debug, Serialize)]
pub struct JoinServerResponse {
    pub server: ServerItem
}

#[derive(Debug, Deserialize, Clone)]
pub struct JoinServerRequest {
    pub invitcode: i32
}

#[derive(Debug, Serialize, Clone)]

pub struct MemberItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub status: String
}

#[derive(Debug, Serialize)]
pub struct GetServerMemberResponse {
    pub members: Vec<MemberItem>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRequest {
    pub new_role: String
}

#[derive(Debug, Serialize)]
pub struct UpdateMemberResponse {
    pub id : Uuid,
    pub new_user: MemberItem
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub description: String
}

#[derive(Debug, Serialize)]
pub struct GetChannelsResponse {
    pub channels: Vec<ChannelItem>
}

#[derive(Debug, Deserialize)]
pub struct BanUserRequest {
    pub duration: Option<i32>
}