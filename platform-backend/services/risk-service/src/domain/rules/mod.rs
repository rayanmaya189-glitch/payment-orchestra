use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::RiskAssessment;
use platform_error::PlatformError;

#[async_trait]
pub trait RiskAssessmentRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskAssessment>, PlatformError>;
    async fn save(&self, assessment: &RiskAssessment) -> Result<(), PlatformError>;
}
