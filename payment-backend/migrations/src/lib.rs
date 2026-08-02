//! Database migrations for Payment Orchestra platform.
//! Uses SeaORM migration framework — each file is one migration version.
//! Run with: `cargo run --bin migrations`

pub use sea_orm_migration::prelude::*;

mod m20260101_000001_create_core_tables;
mod m20260101_000002_create_business_tables;
mod m20260101_000002a_payment_core;
mod m20260101_000002b_identity;
mod m20260101_000002c_gateway_products;
mod m20260101_000003_create_supporting_tables;
mod m20260101_000003a_reconciliation;
mod m20260101_000003b_compliance_notifications;
mod m20260101_000003c_operations;
mod m20260101_000004_saas_billing;
mod m20260101_000005_row_level_security;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260101_000001_create_core_tables::Migration),
            Box::new(m20260101_000002a_payment_core::Migration),
            Box::new(m20260101_000002b_identity::Migration),
            Box::new(m20260101_000002c_gateway_products::Migration),
            Box::new(m20260101_000002_create_business_tables::Migration),
            Box::new(m20260101_000003a_reconciliation::Migration),
            Box::new(m20260101_000003b_compliance_notifications::Migration),
            Box::new(m20260101_000003c_operations::Migration),
            Box::new(m20260101_000003_create_supporting_tables::Migration),
            Box::new(m20260101_000004_saas_billing::Migration),
            Box::new(m20260101_000005_row_level_security::Migration),
        ]
    }
}
