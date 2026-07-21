use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Dispute;
use crate::domain::entities::Evidence;
use platform_error::PlatformError;

/// Core repository for dispute aggregates.
#[async_trait]
pub trait DisputeRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Dispute>, PlatformError>;
    async fn save(&self, dispute: &Dispute) -> Result<(), PlatformError>;
    async fn list_by_operator(&self, operator_id: Uuid) -> Result<Vec<Dispute>, PlatformError>;
}

/// Repository for dispute evidence records.
#[async_trait]
pub trait EvidenceRepository: Send + Sync {
    async fn add_evidence(&self, dispute_id: Uuid, evidence: &Evidence) -> Result<(), PlatformError>;
    async fn get_evidence(&self, dispute_id: Uuid) -> Result<Vec<Evidence>, PlatformError>;
}
