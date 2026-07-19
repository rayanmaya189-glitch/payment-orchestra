use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::{ApiKey, Principal, PendingChange};
use crate::domain::entities::RoleAssignment;
use platform_error::PlatformError;

#[async_trait]
pub trait PrincipalRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<Principal>, PlatformError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<Principal>, PlatformError>;
    async fn save(&self, principal: &Principal) -> Result<(), PlatformError>;
    async fn list_roles(&self, principal_id: Uuid) -> Result<Vec<RoleAssignment>, PlatformError>;
}

#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn save(&self, api_key: &ApiKey) -> Result<(), PlatformError>;
    async fn find_by_key_hash(&self, hash: &[u8]) -> Result<Option<ApiKey>, PlatformError>;
    async fn revoke(&self, id: Uuid) -> Result<(), PlatformError>;
    async fn list_by_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, PlatformError>;
}

#[async_trait]
pub trait PendingChangeRepository: Send + Sync {
    async fn save(&self, change: &PendingChange) -> Result<(), PlatformError>;
    async fn load(&self, id: Uuid) -> Result<Option<PendingChange>, PlatformError>;
}
