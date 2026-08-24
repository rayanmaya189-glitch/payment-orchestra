//! IamRepository trait implementation for PostgresIamRepository.
//!
//! Delegates Principal, ApiKey, and PendingChange operations to the
//! respective sub-modules.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{ApiKey, IamError, PendingChange, Principal};
use super::PostgresIamRepository;
use crate::repository::IamRepository;

#[async_trait]
impl IamRepository for PostgresIamRepository {
    // ─── Principal Operations ─────────────────────────────────────────────

    async fn load_principal(&self, id: Uuid) -> Result<Option<Principal>, IamError> {
        self.load_principal_domain(id).await
    }

    async fn save_principal(&self, principal: &Principal) -> Result<(), IamError> {
        self.save_principal_domain(principal).await
    }

    async fn find_principal_by_email(&self, email: &str) -> Result<Option<Principal>, IamError> {
        self.find_principal_by_email_domain(email).await
    }

    // ─── ApiKey Operations ────────────────────────────────────────────────

    async fn load_api_key(&self, id: Uuid) -> Result<Option<ApiKey>, IamError> {
        self.load_api_key_domain(id).await
    }

    async fn save_api_key(&self, key: &ApiKey) -> Result<(), IamError> {
        self.save_api_key_domain(key).await
    }

    async fn list_api_keys_for_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError> {
        self.list_api_keys_for_principal_domain(principal_id).await
    }

    async fn find_api_key_by_name(&self, principal_id: Uuid, name: &str) -> Result<Option<ApiKey>, IamError> {
        self.find_api_key_by_name_domain(principal_id, name).await
    }

    // ─── PendingChange Operations ─────────────────────────────────────────

    async fn load_change(&self, id: Uuid) -> Result<Option<PendingChange>, IamError> {
        self.load_change_domain(id).await
    }

    async fn save_change(&self, change: &PendingChange) -> Result<(), IamError> {
        self.save_change_domain(change).await
    }
}
