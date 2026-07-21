use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::SettlementBatch;
use crate::domain::value_objects::SettlementRecord;
use platform_error::PlatformError;

/// Repository for settlement batch aggregates.
#[async_trait]
pub trait SettlementBatchRepository: Send + Sync {
    /// Find a batch by ID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SettlementBatch>, PlatformError>;

    /// Upsert a batch (insert or update).
    async fn save(&self, batch: &SettlementBatch) -> Result<(), PlatformError>;

    /// Find batches by operator_id, ordered by created_at descending.
    async fn find_by_operator(
        &self,
        operator_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<SettlementBatch>, PlatformError>;

    /// Find batches by status.
    async fn find_by_status(
        &self,
        status: &str,
        limit: u64,
    ) -> Result<Vec<SettlementBatch>, PlatformError>;
}

/// Repository for settlement records (parsed from connector files).
#[async_trait]
pub trait SettlementRecordRepository: Send + Sync {
    /// Save parsed settlement records for a batch.
    async fn save_records(
        &self,
        batch_id: Uuid,
        records: &[SettlementRecord],
    ) -> Result<(), PlatformError>;

    /// Load settlement records for a batch.
    async fn load_records(
        &self,
        batch_id: Uuid,
    ) -> Result<Vec<SettlementRecord>, PlatformError>;

    /// Save match results for individual records.
    async fn save_match_results(
        &self,
        batch_id: Uuid,
        results: &[crate::domain::value_objects::MatchResult],
    ) -> Result<(), PlatformError>;
}

/// Repository for internal ledger entries.
#[async_trait]
pub trait LedgerRepository: Send + Sync {
    /// Append a ledger entry.
    async fn append(&self, entry: &crate::domain::aggregates::LedgerEntry)
        -> Result<(), PlatformError>;

    /// Find ledger entries by acquirer reference (used for matching).
    async fn find_by_acquirer_reference(
        &self,
        acquirer_reference: &str,
    ) -> Result<Vec<crate::domain::aggregates::LedgerEntry>, PlatformError>;

    /// Find all ledger entries in a period for reconciliation.
    async fn find_in_period(
        &self,
        start: &str,
        end: &str,
        connector_id: &str,
    ) -> Result<Vec<crate::domain::aggregates::LedgerEntry>, PlatformError>;

    /// Mark entries as reconciled with a batch ID.
    async fn mark_reconciled(
        &self,
        entry_ids: &[Uuid],
        batch_id: Uuid,
    ) -> Result<(), PlatformError>;

    /// Verify ledger balance for a transaction (INV-10).
    async fn verify_balance(
        &self,
        transaction_id: Uuid,
    ) -> Result<bool, PlatformError>;

    /// Find transactions with imbalanced ledger entries.
    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, PlatformError>;
}

/// Idempotency guard to prevent duplicate batch ingestion (INV-06).
#[async_trait]
pub trait BatchIdempotencyGuard: Send + Sync {
    /// Check if a batch with this checksum has already been ingested.
    async fn is_duplicate(&self, checksum: &str) -> Result<bool, PlatformError>;

    /// Record a batch checksum as ingested.
    async fn record_checksum(&self, batch_id: Uuid, checksum: &str) -> Result<(), PlatformError>;
}
