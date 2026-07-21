//! Webhook notification provider — delivers JSON payloads to subscriber URLs.
//!
//! Signs each payload with HMAC-SHA256 so receivers can verify authenticity.
//! Uses `reqwest` for HTTP transport and supports configurable timeouts
//! and retry semantics via the `ProviderError` type.

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::domain::rules::{ProviderError, ProviderResult, WebhookProvider};

type HmacSha256 = Hmac<Sha256>;

/// Configuration for the webhook notification provider.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// HMAC secret used to sign payloads (base64-encoded or raw bytes).
    pub signing_secret: String,
    /// HTTP request timeout in seconds.
    pub timeout_secs: u64,
    /// Optional custom header name for the signature (default: `X-Webhook-Signature`).
    pub signature_header: String,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            signing_secret: String::new(),
            timeout_secs: 10,
            signature_header: "X-Webhook-Signature".to_string(),
        }
    }
}

/// Production webhook provider — POSTs JSON to subscriber URLs with HMAC-SHA256 signature.
pub struct WebhookNotificationProvider {
    config: WebhookConfig,
    http_client: reqwest::Client,
}

impl WebhookNotificationProvider {
    pub fn new(config: WebhookConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("Failed to create HTTP client for webhooks");

        Self {
            config,
            http_client,
        }
    }

    /// Compute HMAC-SHA256 signature of the raw JSON body.
    ///
    /// The signature is hex-encoded and sent in the configured header.
    /// Receivers should verify it against their copy of the shared secret.
    fn sign_payload(&self, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(self.config.signing_secret.as_bytes())
                .expect("HMAC accepts any key length");
        mac.update(body.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }
}

#[async_trait]
impl WebhookProvider for WebhookNotificationProvider {
    /// POST a JSON payload to the webhook URL with HMAC-SHA256 signature.
    ///
    /// Sends:
    /// ```http
    /// POST {url} HTTP/1.1
    /// Content-Type: application/json
    /// X-Webhook-Signature: sha256={hex_signature}
    /// X-Request-Id: {uuid}
    /// ```
    ///
    /// The body is the canonical JSON serialization of `payload`.
    ///
    /// # Errors
    /// - `ProviderError::Transient` — network error, timeout, or 5xx response.
    /// - `ProviderError::Permanent` — 4xx response (invalid URL, auth failure).
    /// - `ProviderError::Unavailable` — DNS resolution failure or connection refused.
    async fn send_webhook(
        &self,
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<ProviderResult, ProviderError> {
        let body = serde_json::to_string(payload).map_err(|e| {
            ProviderError::Permanent(format!("Failed to serialize webhook payload: {e}"))
        })?;

        let signature = self.sign_payload(&body);
        let request_id = uuid::Uuid::now_v7().to_string();

        tracing::info!(
            url = url,
            request_id = %request_id,
            body_size = body.len(),
            "Sending webhook POST with HMAC-SHA256 signature"
        );

        let response = self
            .http_client
            .post(url)
            .header("Content-Type", "application/json")
            .header(&self.config.signature_header, format!("sha256={signature}"))
            .header("X-Request-Id", &request_id)
            .body(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    ProviderError::Transient(format!("Webhook connection failed: {e}"))
                } else if e.is_request() {
                    ProviderError::Unavailable(format!("Webhook request error: {e}"))
                } else {
                    ProviderError::Transient(format!("Webhook send error: {e}"))
                }
            })?;

        let status = response.status();

        if status.is_success() {
            tracing::info!(
                url = url,
                request_id = %request_id,
                status = status.as_u16(),
                "Webhook delivered successfully"
            );

            Ok(ProviderResult {
                message_id: format!("wh_{}", request_id),
                metadata: Some(serde_json::json!({
                    "url": url,
                    "status": status.as_u16(),
                    "signature": format!("sha256={}", &signature[..16]),
                })),
            })
        } else if status.is_client_error() {
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(
                url = url,
                status = status.as_u16(),
                body = %error_body,
                "Webhook rejected by subscriber"
            );
            Err(ProviderError::Permanent(format!(
                "Webhook subscriber returned {status}: {error_body}"
            )))
        } else {
            let error_body = response.text().await.unwrap_or_default();
            tracing::warn!(
                url = url,
                status = status.as_u16(),
                body = %error_body,
                "Webhook subscriber returned server error"
            );
            Err(ProviderError::Transient(format!(
                "Webhook subscriber returned {status}: {error_body}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_defaults() {
        let config = WebhookConfig::default();
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.signature_header, "X-Webhook-Signature");
    }

    #[test]
    fn test_hmac_signature_deterministic() {
        let config = WebhookConfig {
            signing_secret: "test-secret".to_string(),
            ..Default::default()
        };
        let provider = WebhookNotificationProvider::new(config);

        let sig1 = provider.sign_payload(r#"{"event":"test"}"#);
        let sig2 = provider.sign_payload(r#"{"event":"test"}"#);
        assert_eq!(sig1, sig2);
        assert_eq!(sig1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_hmac_signature_differs_for_different_payloads() {
        let config = WebhookConfig {
            signing_secret: "test-secret".to_string(),
            ..Default::default()
        };
        let provider = WebhookNotificationProvider::new(config);

        let sig1 = provider.sign_payload("payload-one");
        let sig2 = provider.sign_payload("payload-two");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_hmac_signature_differs_for_different_secrets() {
        let config1 = WebhookConfig {
            signing_secret: "secret-a".to_string(),
            ..Default::default()
        };
        let config2 = WebhookConfig {
            signing_secret: "secret-b".to_string(),
            ..Default::default()
        };
        let provider1 = WebhookNotificationProvider::new(config1);
        let provider2 = WebhookNotificationProvider::new(config2);

        let sig1 = provider1.sign_payload("same payload");
        let sig2 = provider2.sign_payload("same payload");
        assert_ne!(sig1, sig2);
    }
}
