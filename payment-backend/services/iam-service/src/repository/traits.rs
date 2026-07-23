//! Unified repository trait for all IAM aggregates.

use uuid::Uuid;

use crate::domain::{Principal, PendingChange, ApiKey, IamError};

/// Unified repository trait for all IAM aggregates
#[async_trait::async_trait]
pub trait IamRepository: Send + Sync {
    // Principal operations
    async fn load_principal(&self, id: Uuid) -> Result<Option<Principal>, IamError>;
    async fn save_principal(&self, principal: &Principal) -> Result<(), IamError>;
    async fn find_principal_by_email(&self, email: &str) -> Result<Option<Principal>, IamError>;

    // ApiKey operations
    async fn load_api_key(&self, id: Uuid) -> Result<Option<ApiKey>, IamError>;
    async fn save_api_key(&self, key: &ApiKey) -> Result<(), IamError>;
    async fn list_api_keys_for_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError>;
    async fn find_api_key_by_name(&self, principal_id: Uuid, name: &str) -> Result<Option<ApiKey>, IamError>;

    // PendingChange operations
    async fn load_change(&self, id: Uuid) -> Result<Option<PendingChange>, IamError>;
    async fn save_change(&self, change: &PendingChange) -> Result<(), IamError>;
}
