//! Repository trait definitions for reconciliation-service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

/// Repository for SettlementBatch aggregate (event-sourced).
#[async_trait]
pub trait SettlementBatchRepository: Send + Sync {
    async fn load_settlement_batch(&self, id: Uuid) -> Result<Option<SettlementBatch>, ReconciliationError>;
    async fn save_settlement_batch(&self, batch: &mut SettlementBatch) -> Result<(), ReconciliationError>;
    async fn find_batch_by_checksum(&self, checksum: &str) -> Result<Option<SettlementBatch>, ReconciliationError>;
    async fn list_all_batches(&self) -> Result<Vec<SettlementBatch>, ReconciliationError>;
}

/// Repository for LedgerEntry (append-only).
#[async_trait]
pub trait LedgerEntryRepository: Send + Sync {
    async fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<(), ReconciliationError>;
    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, ReconciliationError>;
    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, ReconciliationError>;
}

/// Repository for SettlementExpectation (CRUD).
#[async_trait]
pub trait SettlementExpectationRepository: Send + Sync {
    async fn save_settlement_expectation(&self, expectation: &SettlementExpectation) -> Result<(), ReconciliationError>;
    async fn load_settlement_expectation(&self, id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError>;
    async fn find_expectation_by_payment(&self, payment_intent_id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError>;
    async fn find_overdue_expectations(&self) -> Result<Vec<SettlementExpectation>, ReconciliationError>;
}

/// Repository for FeeVariance (CRUD).
#[async_trait]
pub trait FeeVarianceRepository: Send + Sync {
    async fn save_fee_variance(&self, variance: &FeeVariance) -> Result<(), ReconciliationError>;
    async fn load_fee_variance(&self, id: Uuid) -> Result<Option<FeeVariance>, ReconciliationError>;
    async fn find_fee_variances_for_payment(&self, payment_intent_id: Uuid) -> Result<Vec<FeeVariance>, ReconciliationError>;
}
