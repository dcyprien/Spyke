use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "servers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    
    pub name: String,
    
    pub description: String,
    
    pub icon_url: Option<String>,
    
    pub owner_id: Uuid,  // Le propriétaire unique

    pub invitcode: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id"
    )]
    Owner,
    
    #[sea_orm(has_many = "super::server_member::Entity")]
    Members,
    
    // #[sea_orm(has_many = "super::channel::Entity")]
    // Channels,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl Related<super::server_member::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Members.def()
    }
}

// impl Related<super::channel::Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::Channels.def()
//     }
// }

impl ActiveModelBehavior for ActiveModel {}