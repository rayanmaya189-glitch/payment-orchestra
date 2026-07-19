use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::GatewayProfile;
use platform_error::PlatformError;

#[async_trait]
pub trait GatewayProfileRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, PlatformError>;
    async fn save(&self, profile: &GatewayProfile) -> Result<(), PlatformError>;
    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, PlatformError>;
    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, PlatformError>;
    async fn find_by_link(&self, link_id: Uuid) -> Result<Option<GatewayProfile>, PlatformError>;
}
