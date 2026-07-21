use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::SagaInstance;
use platform_error::PlatformError;

#[async_trait]
pub trait SagaRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SagaInstance>, PlatformError>;
    async fn save(&self, saga: &SagaInstance) -> Result<(), PlatformError>;
    async fn find_by_type_and_status(
        &self,
        saga_type: &str,
        status: &str,
        limit: u32,
    ) -> Result<Vec<SagaInstance>, PlatformError>;
}
