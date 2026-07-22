//! Repository interfaces and in-memory implementation for reconciliation-service.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

// ─── Repository Traits ───────────────────────────────────────────────────────

#[async_trait]
pub trait SettlementBatchRepository: Send + Sync {
    async fn load_settlement_batch(&self, id: Uuid) -> Result<Option<SettlementBatch>, ReconciliationError>;
    async fn save_settlement_batch(&self, batch: &SettlementBatch) -> Result<(), ReconciliationError>;
    async fn find_batch_by_checksum(&self, checksum: &str) -> Result<Option<SettlementBatch>, ReconciliationError>;
}

#[async_trait]
pub trait LedgerEntryRepository: Send + Sync {
    async fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<(), ReconciliationError>;
    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, ReconciliationError>;
    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, ReconciliationError>;
}

#[async_trait]
pub trait SettlementExpectationRepository: Send + Sync {
    async fn save_settlement_expectation(&self, expectation: &SettlementExpectation) -> Result<(), ReconciliationError>;
    async fn load_settlement_expectation(&self, id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError>;
    async fn find_expectation_by_payment(&self, payment_intent_id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError>;
    async fn find_overdue_expectations(&self) -> Result<Vec<SettlementExpectation>, ReconciliationError>;
}

#[async_trait]
pub trait FeeVarianceRepository: Send + Sync {
    async fn save_fee_variance(&self, variance: &FeeVariance) -> Result<(), ReconciliationError>;
    async fn load_fee_variance(&self, id: Uuid) -> Result<Option<FeeVariance>, ReconciliationError>;
    async fn find_fee_variances_for_payment(&self, payment_intent_id: Uuid) -> Result<Vec<FeeVariance>, ReconciliationError>;
}

// ─── In-Memory Implementation ────────────────────────────────────────────────

#[derive(Clone)]
pub struct InMemoryReconciliationRepository {
    batches: Arc<RwLock<HashMap<Uuid, SettlementBatch>>>,
    batch_checksums: Arc<RwLock<HashMap<String, Uuid>>>,
    ledger: Arc<RwLock<Vec<LedgerEntry>>>,
    expectations: Arc<RwLock<HashMap<Uuid, SettlementExpectation>>>,
    fee_variances: Arc<RwLock<HashMap<Uuid, FeeVariance>>>,
}

impl InMemoryReconciliationRepository {
    pub fn new() -> Self {
        Self {
            batches: Arc::new(RwLock::new(HashMap::new())),
            batch_checksums: Arc::new(RwLock::new(HashMap::new())),
            ledger: Arc::new(RwLock::new(Vec::new())),
            expectations: Arc::new(RwLock::new(HashMap::new())),
            fee_variances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn seed_batch(&self, batch: SettlementBatch) {
        let checksum = batch.file_checksum.clone();
        let batch_id = batch.settlement_batch_id;
        self.batches.write().await.insert(batch_id, batch);
        self.batch_checksums.write().await.insert(checksum, batch_id);
    }
}

#[async_trait]
impl SettlementBatchRepository for InMemoryReconciliationRepository {
    async fn load_settlement_batch(&self, id: Uuid) -> Result<Option<SettlementBatch>, ReconciliationError> {
        let store = self.batches.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn save_settlement_batch(&self, batch: &SettlementBatch) -> Result<(), ReconciliationError> {
        let checksum = batch.file_checksum.clone();
        let batch_id = batch.settlement_batch_id;
        self.batches.write().await.insert(batch_id, batch.clone());
        self.batch_checksums.write().await.insert(checksum, batch_id);
        Ok(())
    }

    async fn find_batch_by_checksum(&self, checksum: &str) -> Result<Option<SettlementBatch>, ReconciliationError> {
        let checksums = self.batch_checksums.read().await;
        if let Some(batch_id) = checksums.get(checksum) {
            let store = self.batches.read().await;
            return Ok(store.get(batch_id).cloned());
        }
        Ok(None)
    }
}

#[async_trait]
impl LedgerEntryRepository for InMemoryReconciliationRepository {
    async fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<(), ReconciliationError> {
        self.ledger.write().await.push(entry.clone());
        Ok(())
    }

    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, ReconciliationError> {
        let ledger = self.ledger.read().await;
        let total_debit: i64 = ledger.iter()
            .filter(|e| e.transaction_id == transaction_id && e.entry_type == EntryType::Debit)
            .map(|e| e.amount_minor)
            .sum();
        let total_credit: i64 = ledger.iter()
            .filter(|e| e.transaction_id == transaction_id && e.entry_type == EntryType::Credit)
            .map(|e| e.amount_minor)
            .sum();
        Ok(total_debit == total_credit)
    }

    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, ReconciliationError> {
        let ledger = self.ledger.read().await;
        let mut txn_ids: Vec<Uuid> = ledger.iter().map(|e| e.transaction_id).collect();
        txn_ids.sort();
        txn_ids.dedup();

        let mut imbalanced = Vec::new();
        for txn_id in txn_ids {
            let total_debit: i64 = ledger.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == EntryType::Debit)
                .map(|e| e.amount_minor)
                .sum();
            let total_credit: i64 = ledger.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == EntryType::Credit)
                .map(|e| e.amount_minor)
                .sum();
            if total_debit != total_credit {
                imbalanced.push(txn_id);
            }
        }
        Ok(imbalanced)
    }
}

#[async_trait]
impl SettlementExpectationRepository for InMemoryReconciliationRepository {
    async fn save_settlement_expectation(&self, expectation: &SettlementExpectation) -> Result<(), ReconciliationError> {
        self.expectations.write().await.insert(expectation.expectation_id, expectation.clone());
        Ok(())
    }

    async fn load_settlement_expectation(&self, id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let store = self.expectations.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn find_expectation_by_payment(&self, payment_intent_id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let store = self.expectations.read().await;
        Ok(store.values().find(|e| e.payment_intent_id == payment_intent_id).cloned())
    }

    async fn find_overdue_expectations(&self) -> Result<Vec<SettlementExpectation>, ReconciliationError> {
        let store = self.expectations.read().await;
        let now = Utc::now();
        Ok(store.values()
            .filter(|e| e.status == ExpectationStatus::Pending && e.expected_settlement_date < now)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl FeeVarianceRepository for InMemoryReconciliationRepository {
    async fn save_fee_variance(&self, variance: &FeeVariance) -> Result<(), ReconciliationError> {
        self.fee_variances.write().await.insert(variance.variance_id, variance.clone());
        Ok(())
    }

    async fn load_fee_variance(&self, id: Uuid) -> Result<Option<FeeVariance>, ReconciliationError> {
        let store = self.fee_variances.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn find_fee_variances_for_payment(&self, payment_intent_id: Uuid) -> Result<Vec<FeeVariance>, ReconciliationError> {
        let store = self.fee_variances.read().await;
        Ok(store.values().filter(|v| v.payment_intent_id == payment_intent_id).cloned().collect())
    }
}
