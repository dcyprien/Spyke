use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Servers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Servers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Servers::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Servers::Invitcode)
                            .integer()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Servers::Description)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Servers::IconUrl)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Servers::OwnerId)
                            .uuid()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_servers_owner_id")
                            .from(Servers::Table, Servers::OwnerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Servers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
    Name,
    Description,
    Invitcode,
    IconUrl,
    OwnerId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
