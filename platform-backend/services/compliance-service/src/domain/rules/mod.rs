use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::KybCase;
use platform_error::PlatformError;

#[async_trait]
pub trait KybCaseRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<KybCase>, PlatformError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, PlatformError>;
    async fn save(&self, case: &KybCase) -> Result<(), PlatformError>;
    async fn list_pending(&self) -> Result<Vec<KybCase>, PlatformError>;
}
