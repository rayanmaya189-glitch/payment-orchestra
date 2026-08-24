//! AcquirerLinkProvider implementation for InMemoryOrchestrationRepository.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryOrchestrationRepository;
use super::traits::AcquirerLinkProvider;

#[async_trait]
impl AcquirerLinkProvider for InMemoryOrchestrationRepository {
    async fn list_active_acquirer_links(&self, operator_id: Uuid) -> Result<Vec<Uuid>, OrchestrationError> {
        let links = self.active_links.read().await;
        Ok(links.get(&operator_id).cloned().unwrap_or_default())
    }

    async fn get_acquirer_link_connector(&self, link_id: Uuid) -> Result<String, OrchestrationError> {
        let map = self.link_connector_map.read().await;
        map.get(&link_id)
            .cloned()
            .ok_or_else(|| OrchestrationError::NotFound(link_id))
    }
}
