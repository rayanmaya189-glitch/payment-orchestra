//! SettlementBatchRepository implementation for InMemoryReconciliationRepository.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryReconciliationRepository;
use super::traits::SettlementBatchRepository;

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

    async fn list_all_batches(&self) -> Result<Vec<SettlementBatch>, ReconciliationError> {
        let store = self.batches.read().await;
        let mut batches: Vec<SettlementBatch> = store.values().cloned().collect();
        batches.sort_by_key(|a| a.ingested_at);
        Ok(batches)
    }
}
