use async_trait::async_trait;
use crate::domain::aggregates::AiRequest;
use platform_error::PlatformError;

#[async_trait]
pub trait AiRequestRepository: Send + Sync {
    async fn save(&self, request: &AiRequest) -> Result<(), PlatformError>;
}
