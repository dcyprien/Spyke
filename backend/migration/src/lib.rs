pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table_user;
mod m20220101_000002_create_channels;
mod m20220101_000003_create_table_message;
mod m20260128_145213_create_refresh_tokens;
mod m20260129_084245_create_servers;
mod m20260129_092557_create_server_member;
mod m20260309_143110_create_server_bans;
mod m20260310_092241_create_direct_message;
mod m20260311_000001_create_message_reactions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table_user::Migration),
            Box::new(m20260129_084245_create_servers::Migration),
            Box::new(m20220101_000002_create_channels::Migration),
            Box::new(m20220101_000003_create_table_message::Migration),
            Box::new(m20260128_145213_create_refresh_tokens::Migration),
            Box::new(m20260129_092557_create_server_member::Migration),
            Box::new(m20260309_143110_create_server_bans::Migration),
            Box::new(m20260310_092241_create_direct_message::Migration),
            Box::new(m20260311_000001_create_message_reactions::Migration),
        ]
    }
}
