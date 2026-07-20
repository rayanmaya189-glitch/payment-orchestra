use async_trait::async_trait;
use crate::domain::rules::EmailProvider;
use platform_error::PlatformError;

pub struct MockEmailProvider;
impl MockEmailProvider { pub fn new() -> Self { Self } }

#[async_trait]
impl EmailProvider for MockEmailProvider {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<String, PlatformError> {
        tracing::info!(to = to, subject = subject, "Mock email sent");
        Ok(format!("mock_msg_{}", uuid::Uuid::now_v7()))
    }
}
