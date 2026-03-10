use sea_orm_migration::{prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DirectMessages::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(DirectMessages::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(DirectMessages::Content).text().not_null())
                    .col(ColumnDef::new(DirectMessages::SenderId).uuid().not_null())
                    .col(ColumnDef::new(DirectMessages::ReceiverId).uuid().not_null())
                    .col(ColumnDef::new(DirectMessages::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_direct_messages_sender_id")
                            .from(DirectMessages::Table, DirectMessages::SenderId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_direct_messages_receiver_id")
                            .from(DirectMessages::Table, DirectMessages::ReceiverId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DirectMessages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DirectMessages {
    #[sea_orm(iden = "direct_messages")]
    Table,
    Id,
    Content,
    SenderId,
    ReceiverId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    #[sea_orm(iden = "users")]
    Table,
    Id,
}
