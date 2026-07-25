//! PostgreSQL-backed gateway profile repository using SeaORM.
//!
//! Implements the GatewayProfileRepository trait for production use.
//! Volume tracking fields (daily_volume, monthly_volume, success_rate)
//! use in-memory stores since they're operational metrics not persisted in SQL.

use std::collections::HashMap;
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod gateway_profile;

#[cfg(test)]
pub(crate) mod tests;

/// Combined PostgreSQL-backed repository implementing GatewayProfileRepository.
#[derive(Clone)]
pub struct PostgresConnectorGatewayRepository {
    pub db: DatabaseConnection,
    // In-memory operational metrics (volume tracking, success rates)
    pub(super) daily_volumes: Arc<RwLock<HashMap<Uuid, i64>>>,
    pub(super) monthly_volumes: Arc<RwLock<HashMap<Uuid, i64>>>,
    pub(super) success_counts: Arc<RwLock<HashMap<Uuid, (u32, u32)>>>,
}

impl PostgresConnectorGatewayRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            daily_volumes: Arc::new(RwLock::new(HashMap::new())),
            monthly_volumes: Arc::new(RwLock::new(HashMap::new())),
            success_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
