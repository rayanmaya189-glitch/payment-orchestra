use async_trait::async_trait;

use crate::domain::rules::{ProviderError, ProviderResult, SmsProvider};

/// Configuration for the SMS provider stub.
///
/// In production, replace this with Twilio, Vonage, or AWS SNS integration.
/// The stub logs the SMS and returns a synthetic message ID.
#[derive(Debug, Clone)]
pub struct SmsStubConfig {
    /// API endpoint URL (reserved for future real integration).
    pub api_url: String,
    /// API key (reserved for future real integration).
    pub api_key: String,
    /// Sender phone number or short code.
    pub sender_id: String,
}

/// SMS provider stub. Logs SMS content and returns a synthetic message ID.
///
/// Replace with real provider (Twilio/Vonage/SNS) by implementing the `SmsProvider` trait
/// with actual HTTP calls.
pub struct SmsProviderStub {
    config: SmsStubConfig,
}

impl SmsProviderStub {
    pub fn new(config: SmsStubConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SmsProvider for SmsProviderStub {
    async fn send_sms(&self, to: &str, body: &str) -> Result<ProviderResult, ProviderError> {
        tracing::info!(
            to = to,
            sender = %self.config.sender_id,
            body_len = body.len(),
            "SMS dispatched via stub provider"
        );

        let message_id = format!("sms_stub_{}", uuid::Uuid::now_v7());

        Ok(ProviderResult {
            message_id,
            metadata: Some(serde_json::json!({
                "provider": "stub",
                "sender_id": self.config.sender_id,
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sms_stub_sends() {
        let config = SmsStubConfig {
            api_url: "https://sms.example.com".into(),
            api_key: "test_key".into(),
            sender_id: "PayPlatform".into(),
        };
        let stub = SmsProviderStub::new(config);
        let result = stub.send_sms("+971501234567", "Your code is 1234").await.unwrap();
        assert!(result.message_id.starts_with("sms_stub_"));
    }
}
