pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_operators;
mod m20240101_000002_create_principals;
mod m20240101_000003_create_api_keys;
mod m20240101_000004_create_pending_changes;
mod m20240101_000005_create_kyb_cases;
mod m20240101_000006_create_outbox;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_operators::Migration),
            Box::new(m20240101_000002_create_principals::Migration),
            Box::new(m20240101_000003_create_api_keys::Migration),
            Box::new(m20240101_000004_create_pending_changes::Migration),
            Box::new(m20240101_000005_create_kyb_cases::Migration),
            Box::new(m20240101_000006_create_outbox::Migration),
        ]
    }
}
