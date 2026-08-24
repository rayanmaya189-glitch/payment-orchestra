//! PostgreSQL-backed reconciliation repositories using SeaORM + platform-db entities.
//!
//! Implements all 4 repository traits: SettlementBatchRepository, LedgerEntryRepository,
//! SettlementExpectationRepository, FeeVarianceRepository.

use sea_orm::DatabaseConnection;

pub mod settlement_batch;
pub mod ledger_entry;
pub mod settlement_expectation;
pub mod fee_variance;

/// Combined PostgreSQL-backed repository implementing all 4 reconciliation traits.
pub struct PostgresReconciliationRepository {
    pub db: DatabaseConnection,
}

impl PostgresReconciliationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
