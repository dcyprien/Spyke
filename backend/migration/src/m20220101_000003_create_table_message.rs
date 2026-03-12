use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Messages::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Messages::Content).text().not_null())
                    .col(ColumnDef::new(Messages::UserId).uuid().not_null())
                    .col(ColumnDef::new(Messages::ChannelId).uuid().null())
                    .col(ColumnDef::new(Messages::ServerId).integer().null())
                    .col(ColumnDef::new(Messages::DirectMessage).uuid().null())
                    .col(
                        ColumnDef::new(Messages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Clés étrangères (Foreign Keys)
                    // Attention: il faut que la table 'channels' existe déjà !
                    // Si tu n'as pas encore fait Channel, commente la partie FK Channel pour l'instant
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-message-user")
                            .from(Messages::Table, Messages::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-message-channel")
                            .from(Messages::Table, Messages::ChannelId)
                            .to(Channels::Table, Channels::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // CRÉATION DE L'INDEX (Optionnel mais recommandé "Senior Dev")
        // Pour charger l'historique d'un channel ultra vite
        manager
            .create_index(
                Index::create()
                    .name("idx-message-channel-created")
                    .table(Messages::Table)
                    .col(Messages::ChannelId)
                    .col(Messages::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await
    }
}

// Les IDEN (noms pour éviter les typos)
#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    Content,
    UserId,
    ServerId,
    ChannelId,
    DirectMessage,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Channels {
    Table,
    Id,
}