use async_trait::async_trait;
use uuid::Uuid;

use super::PostgresOrchestrationRepository;
use crate::repository::AcquirerLinkProvider;
use crate::domain::*;

#[async_trait]
impl AcquirerLinkProvider for PostgresOrchestrationRepository {
    async fn list_active_acquirer_links(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<Uuid>, OrchestrationError> {
        let links = self.active_links.read().await;
        Ok(links.get(&operator_id).cloned().unwrap_or_default())
    }
}
