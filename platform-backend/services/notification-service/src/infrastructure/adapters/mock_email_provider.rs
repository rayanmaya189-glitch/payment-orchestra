use async_trait::async_trait;

use crate::domain::rules::{EmailProvider, ProviderError, ProviderResult};

/// Mock email provider for testing. Always succeeds.
pub struct MockEmailProvider;
impl MockEmailProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EmailProvider for MockEmailProvider {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        _body: &str,
    ) -> Result<ProviderResult, ProviderError> {
        tracing::info!(to = to, subject = subject, "Mock email sent");
        Ok(ProviderResult {
            message_id: format!("mock_email_{}", uuid::Uuid::now_v7()),
            metadata: None,
        })
    }
}
