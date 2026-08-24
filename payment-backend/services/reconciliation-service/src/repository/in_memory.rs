//! In-memory implementations of reconciliation repository traits.
//!
//! Fields are `pub(super)` to allow impl blocks in sibling modules
//! to access them for the in-memory backing stores.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

#[derive(Clone)]
pub struct InMemoryReconciliationRepository {
    pub(super) batches: Arc<RwLock<HashMap<Uuid, SettlementBatch>>>,
    pub(super) batch_checksums: Arc<RwLock<HashMap<String, Uuid>>>,
    pub(super) ledger: Arc<RwLock<Vec<LedgerEntry>>>,
    pub(super) expectations: Arc<RwLock<HashMap<Uuid, SettlementExpectation>>>,
    pub(super) fee_variances: Arc<RwLock<HashMap<Uuid, FeeVariance>>>,
}

impl Default for InMemoryReconciliationRepository {
    fn default() -> Self {
        Self::new()
    }
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
