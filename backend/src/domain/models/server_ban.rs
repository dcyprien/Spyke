use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "server_bans")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    pub server_id: i32,
    
    pub user_id: Uuid,
    
    pub banned_by: Uuid,
    
    pub banned_until: Option<chrono::NaiveDateTime>,
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

// Optionnel: Relation vers l'utilisateur (celui qui est banni)
impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}