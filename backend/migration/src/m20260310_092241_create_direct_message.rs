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
                    .col(ColumnDef::new(DirectMessages::User1Id).uuid().not_null()) // Remplacement
                    .col(ColumnDef::new(DirectMessages::User2Id).uuid().not_null()) // Remplacement
                    .col(ColumnDef::new(DirectMessages::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_direct_messages_user1_id")
                            .from(DirectMessages::Table, DirectMessages::User1Id)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_direct_messages_user2_id")
                            .from(DirectMessages::Table, DirectMessages::User2Id)
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
    User1Id, // Remplacement
    User2Id, // Remplacement
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    #[sea_orm(iden = "users")]
    Table,
    Id,
}
