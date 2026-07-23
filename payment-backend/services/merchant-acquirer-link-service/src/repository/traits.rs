//! Repository trait for MerchantAcquirerLink aggregate.

use uuid::Uuid;

use crate::domain::{MerchantAcquirerLink, LinkError};

#[allow(dead_code)]
#[async_trait::async_trait]
pub trait LinkRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, LinkError>;
    async fn save(&self, link: &MerchantAcquirerLink) -> Result<(), LinkError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError>;
    async fn find_active_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError>;
    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<MerchantAcquirerLink>, LinkError>;
    async fn find_by_credentials_hash(&self, hash: &str) -> Result<Option<MerchantAcquirerLink>, LinkError>;
    async fn find_expired_credentials(&self) -> Result<Vec<MerchantAcquirerLink>, LinkError>;
    async fn count_active_by_connector(&self, operator_id: Uuid, connector_id: &str) -> Result<usize, LinkError>;
}
