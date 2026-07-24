//! Migration M003 — Supporting Domain Tables (no-op)
//!
//! Original content split into:
//! - m20260101_000003a_reconciliation.rs
//! - m20260101_000003b_compliance_notifications.rs
//! - m20260101_000003c_operations.rs

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
