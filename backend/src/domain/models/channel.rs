use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "channels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    pub server_id: i32,
    
    pub name: String,
    
    pub description: String,
        
    pub position: i32,
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
    
    #[sea_orm(has_many = "super::message::Entity")]
    Messages,
}

impl Related<super::server_model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Server.def()
    }
}

impl Related<super::message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Messages.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}