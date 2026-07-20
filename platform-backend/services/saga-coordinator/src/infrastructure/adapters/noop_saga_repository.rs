use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::SagaInstance;
use crate::domain::rules::SagaRepository;
use platform_error::PlatformError;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct NoopSagaRepository {
    store: RwLock<HashMap<Uuid, SagaInstance>>,
}

impl NoopSagaRepository {
    pub fn new() -> Self {
        Self { store: RwLock::new(HashMap::new()) }
    }
}

#[async_trait]
impl SagaRepository for NoopSagaRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SagaInstance>, PlatformError> {
        Ok(self.store.read().unwrap().get(&id).cloned())
    }
    async fn save(&self, saga: &SagaInstance) -> Result<(), PlatformError> {
        self.store.write().unwrap().insert(saga.saga_id, saga.clone());
        Ok(())
    }
}
