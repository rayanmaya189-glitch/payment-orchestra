//! FeeVarianceRepository implementation for InMemoryReconciliationRepository.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryReconciliationRepository;
use super::traits::FeeVarianceRepository;

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
