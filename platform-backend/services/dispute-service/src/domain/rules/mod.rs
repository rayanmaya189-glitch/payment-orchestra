use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Dispute;
use platform_error::PlatformError;
#[async_trait]
pub trait DisputeRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Dispute>, PlatformError>;
    async fn save(&self, dispute: &Dispute) -> Result<(), PlatformError>;
}
