use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::SagaInstance;
use crate::domain::rules::SagaRepository;
use platform_error::PlatformError;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory repository for testing.
pub struct NoopSagaRepository {
    store: RwLock<HashMap<Uuid, SagaInstance>>,
}

impl NoopSagaRepository {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SagaRepository for NoopSagaRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SagaInstance>, PlatformError> {
        Ok(self.store.read().unwrap().get(&id).cloned())
    }

    async fn save(&self, saga: &SagaInstance) -> Result<(), PlatformError> {
        self.store
            .write()
            .unwrap()
            .insert(saga.saga_id, saga.clone());
        Ok(())
    }

    async fn find_by_type_and_status(
        &self,
        saga_type: &str,
        status: &str,
        limit: u32,
    ) -> Result<Vec<SagaInstance>, PlatformError> {
        let store = self.store.read().unwrap();
        let results: Vec<SagaInstance> = store
            .values()
            .filter(|s| s.saga_type == saga_type && s.status.as_str() == status)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(results)
    }
}
