//! PostgreSQL-backed compliance repository using SeaORM.
//!
//! Implements the ComplianceRepository trait for KYB case management
//! and AML alert tracking. Transaction history queries use in-memory
//! stores since they're operational metrics not persisted in SQL.

use std::collections::HashMap;
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;

pub mod kyb_case;
pub mod aml_alert;
pub mod repository;

/// Combined PostgreSQL-backed repository implementing ComplianceRepository.
#[derive(Clone)]
pub struct PostgresComplianceRepository {
    pub db: DatabaseConnection,
    // In-memory transaction history for AML scanning (not persisted)
    pub(super) recent_txns: Arc<RwLock<Vec<crate::domain::RecentTransaction>>>,
    // Operator indexed transaction store
    pub(super) operator_txns: Arc<RwLock<HashMap<uuid::Uuid, Vec<crate::domain::RecentTransaction>>>>,
    // Payment-method indexed transaction store
    pub(super) method_txns: Arc<RwLock<HashMap<String, Vec<crate::domain::RecentTransaction>>>>,
}

impl PostgresComplianceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            recent_txns: Arc::new(RwLock::new(Vec::new())),
            operator_txns: Arc::new(RwLock::new(HashMap::new())),
            method_txns: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
