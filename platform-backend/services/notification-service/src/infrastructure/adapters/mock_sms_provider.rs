use async_trait::async_trait;

use crate::domain::rules::{ProviderError, ProviderResult, SmsProvider};

/// Mock SMS provider for testing. Always succeeds.
pub struct MockSmsProvider;
impl MockSmsProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SmsProvider for MockSmsProvider {
    async fn send_sms(&self, to: &str, body: &str) -> Result<ProviderResult, ProviderError> {
        tracing::info!(to = to, body_len = body.len(), "Mock SMS sent");
        Ok(ProviderResult {
            message_id: format!("mock_sms_{}", uuid::Uuid::now_v7()),
            metadata: None,
        })
    }
}
