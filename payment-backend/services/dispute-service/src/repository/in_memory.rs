//! In-memory Dispute Management repository — BC-10

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::DisputeRepository;

#[derive(Clone)]
pub struct InMemoryDisputeRepository {
    pub(super) cases: Arc<RwLock<HashMap<Uuid, ChargebackCase>>>,
}

impl Default for InMemoryDisputeRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDisputeRepository {
    pub fn new() -> Self {
        Self { cases: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl DisputeRepository for InMemoryDisputeRepository {
    async fn load(&self, id: Uuid) -> Result<Option<ChargebackCase>, DisputeError> {
        let map = self.cases.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save(&self, case: &ChargebackCase) -> Result<(), DisputeError> {
        let mut map = self.cases.write().await;
        map.insert(case.chargeback_id, case.clone());
        Ok(())
    }

    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let map = self.cases.read().await;
        let results: Vec<ChargebackCase> = map.values()
            .filter(|c| c.payment_intent_id == payment_intent_id)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_open_cases(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let map = self.cases.read().await;
        let results: Vec<ChargebackCase> = map.values()
            .filter(|c| c.operator_id == operator_id && !c.status.is_resolved())
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let map = self.cases.read().await;
        let results: Vec<ChargebackCase> = map.values()
            .filter(|c| c.operator_id == operator_id)
            .cloned()
            .collect();
        Ok(results)
    }
}
