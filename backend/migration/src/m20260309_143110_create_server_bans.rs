use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerBans::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerBans::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ServerBans::ServerId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerBans::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerBans::BannedBy)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerBans::BannedUntil)
                            .timestamp(), // Nullable
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_bans_server_id")
                            .from(ServerBans::Table, ServerBans::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_bans_user_id")
                            .from(ServerBans::Table, ServerBans::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ServerBans::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerBans {
    Table,
    Id,
    ServerId,
    UserId,
    BannedBy,
    BannedUntil,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
