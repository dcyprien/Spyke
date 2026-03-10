use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "direct_messages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[serde(skip_deserializing)]
    pub id: Uuid,
    
    #[sea_orm(column_type = "Text")]
    pub content: String,

    pub user1_id: Uuid,
    pub user2_id: Uuid,

    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::domain::models::user::Entity",
        from = "Column::User1Id",
        to = "crate::domain::models::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User1,
    
    #[sea_orm(
        belongs_to = "crate::domain::models::user::Entity",
        from = "Column::User2Id",
        to = "crate::domain::models::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User2,
}

impl ActiveModelBehavior for ActiveModel {}