use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // ⬆️ UP : Création de la table
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    // 1. ID (UUID)
                    .col(
                        ColumnDef::new(Users::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    // 2. Identifiants (Uniques)
                    .col(
                        ColumnDef::new(Users::Username)
                            .string()
                            .not_null()
                            .unique_key(), // Important : Pas de doublons
                    )
                    // 3. Sécurité
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    
                    // 4. Profil (Optionnels / Nullable)
                    .col(ColumnDef::new(Users::DisplayName).string().null())
                    .col(ColumnDef::new(Users::AvatarUrl).string().null())
                    
                    // 5. Statut (Enum stocké en String, défaut "Offline")
                    .col(
                        ColumnDef::new(Users::Status)
                            .string()
                            .not_null()
                            .default("Offline"),
                    )
                    // (Pas de timestamps comme tu l'as demandé)
                    .to_owned(),
            )
            .await
    }

    // ⬇️ DOWN : Suppression de la table (rollback)
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

// Définition des noms de colonnes pour éviter les fautes de frappe
#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash,
    DisplayName,
    AvatarUrl,
    Status
}