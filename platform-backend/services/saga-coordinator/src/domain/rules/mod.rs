use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::SagaInstance;
use platform_error::PlatformError;

#[async_trait]
pub trait SagaRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SagaInstance>, PlatformError>;
    async fn save(&self, saga: &SagaInstance) -> Result<(), PlatformError>;
}
