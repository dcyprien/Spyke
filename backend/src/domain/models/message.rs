use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "messages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[serde(skip_deserializing)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text")]
    pub content: String,

    pub user_id: Uuid,

    pub server_id: Option<i32>,

    pub channel_id: Option<Uuid>,

    pub direct_message: Option<Uuid>,

    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // Un message appartient à un User
    #[sea_orm(
        belongs_to = "crate::domain::models::user::Entity",
        from = "Column::UserId",
        to = "crate::domain::models::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,

    // Un message appartient à un Channel
    #[sea_orm(
        belongs_to = "super::channel::Entity",
        from = "Column::ChannelId",
        to = "super::channel::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Channel,
}

// Utile pour faire message.find_related(User)
impl Related<crate::domain::models::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

// Utile pour faire message.find_related(Channel)
impl Related<super::channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Channel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}