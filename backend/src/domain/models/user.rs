use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[serde(skip_deserializing)]
    pub id: Uuid,
    
    #[sea_orm(unique)]
    pub username: String,
    
    #[serde(skip_serializing)]
    pub password_hash: String,
    
    pub display_name: Option<String>,
    
    pub avatar_url: Option<String>,
    
    pub status: UserStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum UserStatus {
    #[sea_orm(string_value = "online")]
    Online,
    #[sea_orm(string_value = "offline")]
    Offline,
    #[sea_orm(string_value = "invisible")]
    Invisible,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::message::Entity")]
    Messages,
    
    #[sea_orm(has_many = "super::server_model::Entity")]
    ServerMemberships,
    
    #[sea_orm(has_many = "super::server_model::Entity")]
    OwnedServers,
}

impl Related<super::message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Messages.def()
    }
}

impl Related<super::server_member::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServerMemberships.def()
    }
}

impl Related<super::server_model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OwnedServers.def()
    }
}


impl ActiveModelBehavior for ActiveModel {}