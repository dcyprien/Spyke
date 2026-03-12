use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Channels::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Channels::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Channels::ServerId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Channels::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Channels::Description)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Channels::Position)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_channels_server_id")
                            .from(Channels::Table, Channels::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Channels::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Channels {
    Table,
    Id,
    ServerId,
    Name,
    Description,
    Position,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}
