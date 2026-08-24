//! Migration M002 — Core Business Domain Tables (no-op)
//!
//! Original content split into:
//! - m20260101_000002a_payment_core.rs
//! - m20260101_000002b_identity.rs
//! - m20260101_000002c_gateway_products.rs

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
