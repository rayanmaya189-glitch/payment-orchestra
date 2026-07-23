//! SettlementExpectationRepository implementation for InMemoryReconciliationRepository.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryReconciliationRepository;
use super::traits::SettlementExpectationRepository;

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
