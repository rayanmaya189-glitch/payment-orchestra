pub mod postgres_settlement_repository;
pub mod settlement_parser;

pub use postgres_settlement_repository::PostgresSettlementRepository;

// ==================== In-Memory Adapters (for development/testing) ====================

use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::LedgerEntry;
use crate::domain::rules::*;
use crate::domain::value_objects::SettlementRecord;
use platform_error::PlatformError;

/// In-memory settlement record repository (for dev/testing).
pub struct InMemoryRecordRepository {
    records: Mutex<HashMap<Uuid, Vec<SettlementRecord>>>,
}

impl InMemoryRecordRepository {
    pub fn new() -> Self {
        Self { records: Mutex::new(HashMap::new()) }
    }
}

#[async_trait]
impl SettlementRecordRepository for InMemoryRecordRepository {
    async fn save_records(
        &self,
        batch_id: Uuid,
        records: &[SettlementRecord],
    ) -> Result<(), PlatformError> {
        let mut map = self.records.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        map.insert(batch_id, records.to_vec());
        Ok(())
    }

    async fn load_records(
        &self,
        batch_id: Uuid,
    ) -> Result<Vec<SettlementRecord>, PlatformError> {
        let map = self.records.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        Ok(map.get(&batch_id).cloned().unwrap_or_default())
    }

    async fn save_match_results(
        &self,
        _batch_id: Uuid,
        _results: &[crate::domain::value_objects::MatchResult],
    ) -> Result<(), PlatformError> {
        // In-memory: results are returned in the response, no persistence needed for dev
        Ok(())
    }
}

/// In-memory ledger repository (for dev/testing).
pub struct InMemoryLedgerRepository {
    entries: Mutex<Vec<LedgerEntry>>,
}

impl InMemoryLedgerRepository {
    pub fn new() -> Self {
        Self { entries: Mutex::new(Vec::new()) }
    }
}

#[async_trait]
impl LedgerRepository for InMemoryLedgerRepository {
    async fn append(
        &self,
        entry: &LedgerEntry,
    ) -> Result<(), PlatformError> {
        let mut entries = self.entries.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        entries.push(entry.clone());
        Ok(())
    }

    async fn find_by_acquirer_reference(
        &self,
        acquirer_reference: &str,
    ) -> Result<Vec<LedgerEntry>, PlatformError> {
        let entries = self.entries.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        Ok(entries.iter()
            .filter(|e| e.source_acquirer == acquirer_reference)
            .cloned()
            .collect())
    }

    async fn find_in_period(
        &self,
        _start: &str,
        _end: &str,
        _connector_id: &str,
    ) -> Result<Vec<LedgerEntry>, PlatformError> {
        let entries = self.entries.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        Ok(entries.clone())
    }

    async fn mark_reconciled(
        &self,
        entry_ids: &[Uuid],
        batch_id: Uuid,
    ) -> Result<(), PlatformError> {
        let mut entries = self.entries.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        for entry in entries.iter_mut() {
            if entry_ids.contains(&entry.entry_id) {
                entry.reconciled = true;
                entry.reconciliation_batch_id = Some(batch_id);
            }
        }
        Ok(())
    }

    async fn verify_balance(
        &self,
        transaction_id: Uuid,
    ) -> Result<bool, PlatformError> {
        let entries = self.entries.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        let tx_entries: Vec<&LedgerEntry> = entries.iter()
            .filter(|e| e.transaction_id == transaction_id)
            .collect();

        let total_debit: i64 = tx_entries.iter().map(|e| e.debit_amount_minor_units).sum();
        let total_credit: i64 = tx_entries.iter().map(|e| e.credit_amount_minor_units).sum();

        Ok(total_debit == total_credit)
    }

    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, PlatformError> {
        let entries = self.entries.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        let mut tx_totals: HashMap<Uuid, (i64, i64)> = HashMap::new();
        for entry in entries.iter() {
            let (debit, credit) = tx_totals.entry(entry.transaction_id).or_insert((0, 0));
            *debit += entry.debit_amount_minor_units;
            *credit += entry.credit_amount_minor_units;
        }
        Ok(tx_totals.iter()
            .filter(|(_, (d, c))| d != c)
            .map(|(id, _)| *id)
            .collect())
    }
}

/// In-memory idempotency guard (for dev/testing).
pub struct InMemoryIdempotencyGuard {
    checksums: Mutex<HashMap<String, Uuid>>,
}

impl InMemoryIdempotencyGuard {
    pub fn new() -> Self {
        Self { checksums: Mutex::new(HashMap::new()) }
    }
}

#[async_trait]
impl BatchIdempotencyGuard for InMemoryIdempotencyGuard {
    async fn is_duplicate(&self, checksum: &str) -> Result<bool, PlatformError> {
        let map = self.checksums.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        Ok(map.contains_key(checksum))
    }

    async fn record_checksum(&self, batch_id: Uuid, checksum: &str) -> Result<(), PlatformError> {
        let mut map = self.checksums.lock().map_err(|e| PlatformError::Internal(format!("Lock: {e}")))?;
        map.insert(checksum.to_string(), batch_id);
        Ok(())
    }
}
