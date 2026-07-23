//! In-memory Saga Coordinator repository — BC-17

use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::SagaRepository;

#[derive(Clone)]
pub struct InMemorySagaRepository {
    pub(super) sagas: Arc<RwLock<HashMap<Uuid, SagaInstance>>>,
}

impl InMemorySagaRepository {
    pub fn new() -> Self {
        Self { sagas: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl SagaRepository for InMemorySagaRepository {
    async fn load(&self, id: Uuid) -> Result<Option<SagaInstance>, SagaError> {
        let map = self.sagas.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save(&self, saga: &SagaInstance) -> Result<(), SagaError> {
        let mut map = self.sagas.write().await;
        map.insert(saga.saga_id, saga.clone());
        Ok(())
    }

    async fn find_stuck(&self, timeout_seconds: i64) -> Result<Vec<SagaInstance>, SagaError> {
        let map = self.sagas.read().await;
        let cutoff = Utc::now() - Duration::seconds(timeout_seconds);
        let stuck: Vec<SagaInstance> = map.values()
            .filter(|s| s.status == SagaStatus::Running && s.updated_at < cutoff)
            .cloned()
            .collect();
        Ok(stuck)
    }

    async fn find_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<SagaInstance>, SagaError> {
        let map = self.sagas.read().await;
        let results: Vec<SagaInstance> = map.values()
            .filter(|s| s.aggregate_id == aggregate_id)
            .cloned()
            .collect();
        Ok(results)
    }
}
