use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "server_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    pub server_id: i32,
    
    pub user_id: Uuid,
    
    pub role: MemberRole,
    
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum MemberRole {
    #[sea_orm(string_value = "owner")]
    Owner,      // 1 seul par serveur (dupliqué depuis server.owner_id pour faciliter les requêtes)
    #[sea_orm(string_value = "admin")]
    Admin,      // Plusieurs admins possibles
    #[sea_orm(string_value = "member")]
    Member,     // Membres normaux
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::server_model::Entity",
        from = "Column::ServerId",
        to = "super::server_model::Column::Id",
        on_delete = "Cascade"
    )]
    Server,
    
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::server_model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Server.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}