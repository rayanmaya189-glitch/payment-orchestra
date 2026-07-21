use async_trait::async_trait;

use crate::domain::rules::{ProviderError, ProviderResult, WebhookProvider};

/// Mock webhook provider for testing. Always succeeds.
pub struct MockWebhookProvider;
impl MockWebhookProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WebhookProvider for MockWebhookProvider {
    async fn send_webhook(
        &self,
        url: &str,
        _payload: &serde_json::Value,
    ) -> Result<ProviderResult, ProviderError> {
        tracing::info!(url = url, "Mock webhook sent");
        Ok(ProviderResult {
            message_id: format!("mock_wh_{}", uuid::Uuid::now_v7()),
            metadata: None,
        })
    }
}
