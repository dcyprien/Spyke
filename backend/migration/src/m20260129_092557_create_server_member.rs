use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerMembers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ServerMembers::ServerId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerMembers::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerMembers::Role)
                            .string()
                            .not_null()
                            .default("member"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_members_server_id")
                            .from(ServerMembers::Table, ServerMembers::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_members_user_id")
                            .from(ServerMembers::Table, ServerMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    // ✅ Index unique pour éviter qu'un user rejoigne 2x le même serveur
                    .index(
                        Index::create()
                            .name("idx_server_user_unique")
                            .table(ServerMembers::Table)
                            .col(ServerMembers::ServerId)
                            .col(ServerMembers::UserId)
                            .unique()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ServerMembers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerMembers {
    Table,
    Id,
    ServerId,
    UserId,
    Role,
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
