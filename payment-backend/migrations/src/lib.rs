//! Database migrations for Payment Orchestra platform.
//! Uses SeaORM migration framework — each file is one migration version.
//! Run with: `cargo run --bin migrations`

pub use sea_orm_migration::prelude::*;

mod m20260101_000001_create_core_tables;
mod m20260101_000002_create_business_tables;
mod m20260101_000003_create_supporting_tables;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260101_000001_create_core_tables::Migration),
            Box::new(m20260101_000002_create_business_tables::Migration),
            Box::new(m20260101_000003_create_supporting_tables::Migration),
        ]
    }
}
