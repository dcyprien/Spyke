use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MessageReactions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MessageReactions::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MessageReactions::MessageId).uuid().not_null())
                    .col(ColumnDef::new(MessageReactions::UserId).uuid().not_null())
                    .col(ColumnDef::new(MessageReactions::Emoji).string().not_null())
                    .col(
                        ColumnDef::new(MessageReactions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reaction-message")
                            .from(MessageReactions::Table, MessageReactions::MessageId)
                            .to(Messages::Table, Messages::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reaction-user")
                            .from(MessageReactions::Table, MessageReactions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint: one reaction per (message, user, emoji)
        manager
            .create_index(
                Index::create()
                    .name("idx-reaction-unique")
                    .table(MessageReactions::Table)
                    .col(MessageReactions::MessageId)
                    .col(MessageReactions::UserId)
                    .col(MessageReactions::Emoji)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MessageReactions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MessageReactions {
    Table,
    Id,
    MessageId,
    UserId,
    Emoji,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
