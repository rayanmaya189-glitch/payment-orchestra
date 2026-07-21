use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::AiRequest;
use crate::domain::rules::AiRequestRepository;
use platform_error::PlatformError;

pub struct NoopAiRequestRepository;

impl NoopAiRequestRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AiRequestRepository for NoopAiRequestRepository {
    async fn save(&self, _request: &AiRequest) -> Result<(), PlatformError> {
        Ok(())
    }

    async fn find_by_id(&self, _id: Uuid) -> Result<Option<AiRequest>, PlatformError> {
        Ok(None)
    }

    async fn find_by_principal(
        &self,
        _principal_id: Uuid,
        _limit: u32,
        _offset: u32,
    ) -> Result<Vec<AiRequest>, PlatformError> {
        Ok(vec![])
    }
}
