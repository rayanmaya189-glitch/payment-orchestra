//! PostgreSQL-backed IAM repository using SeaORM.
//!
//! Implements the IamRepository trait for Principal, ApiKey, and PendingChange
//! aggregates. Each aggregate has its own module in this directory.

use sea_orm::DatabaseConnection;

pub mod principal;
pub mod api_key;
pub mod pending_change;
pub mod repository;

/// Combined PostgreSQL-backed repository implementing IamRepository.
#[derive(Clone)]
pub struct PostgresIamRepository {
    pub db: DatabaseConnection,
}

impl PostgresIamRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
