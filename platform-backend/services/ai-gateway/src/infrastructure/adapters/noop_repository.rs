use async_trait::async_trait;
use crate::domain::aggregates::AiRequest;
use crate::domain::rules::AiRequestRepository;
use platform_error::PlatformError;

pub struct NoopAiRequestRepository;
impl NoopAiRequestRepository { pub fn new() -> Self { Self } }

#[async_trait]
impl AiRequestRepository for NoopAiRequestRepository {
    async fn save(&self, _request: &AiRequest) -> Result<(), PlatformError> { Ok(()) }
}
