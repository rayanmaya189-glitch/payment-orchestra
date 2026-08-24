//! Stripe Connector — real HTTP adapter for Stripe's REST API.
//!
//! Implements the `AcquirerConnector` trait by making authenticated HTTP calls
//! to Stripe's public API. Supports all core payment operations, webhook
//! signature verification (HMAC-SHA256), credential validation against
//! Stripe's sandbox, and Stripe-specific decline code mapping.
//!
//! Split into modules for CONVENTIONS.md compliance (<= 300 lines per file):
//! - `stripe_connector.rs` (this file): struct definition, constructor, helpers
//! - `stripe_impl.rs`: AcquirerConnector trait implementation
//! - `stripe_webhook.rs`: webhook signature verification and event parsing
//! - `stripe_credential.rs`: credential validation and connection testing

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use platform_middleware::ssrf::validate_connector_url;

use super::*;

/// Stripe connector for processing payments through Stripe's REST API.
///
/// # Credentials
/// - `secret_key`: Stripe secret key (`sk_test_...` or `sk_live_...`)
/// - `webhook_secret`: Stripe webhook signing secret (`whsec_...`)
pub struct StripeConnector {
    pub(super) secret_key: String,
    pub(super) webhook_secret: String,
    pub(super) environment: String,
    pub(super) base_url: String,
    pub(super) client: reqwest::Client,
    pub(super) circuit_breaker: Mutex<CircuitBreaker>,
    pub(super) decline_table: DeclineMappingTable,
}

impl StripeConnector {
    /// Create a new StripeConnector with configuration from a ConnectorConfig.
    pub fn new(config: &ConnectorConfig) -> Self {
        let secret_key = config.secret_key.clone().unwrap_or_default();
        let webhook_secret = config
            .additional_fields
            .get("webhook_secret")
            .cloned()
            .unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = "https://api.stripe.com".to_string();

        // Validate base_url for SSRF safety (prevents accidentally connecting
        // to internal/private networks even if config is ever made dynamic)
        if let Err(e) = validate_connector_url(&base_url) {
            // In production this would be a startup fatal error, but for now
            // warn and continue since the URL is hardcoded to api.stripe.com.
            tracing::warn!("SSRF validation warning for base_url '{}': {}", base_url, e);
        }

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("PaymentOrchestra/1.0")
            .build()
            .expect("Failed to create HTTP client for Stripe");

        Self {
            secret_key,
            webhook_secret,
            environment,
            base_url,
            client,
            circuit_breaker: Mutex::new(CircuitBreaker::new()),
            decline_table: DeclineMappingTable::new(HashMap::from([
                ("insufficient_funds".into(), "InsufficientFunds".into()),
                ("do_not_honor".into(), "DoNotHonor".into()),
                ("invalid_card_number".into(), "InvalidCard".into()),
                ("expired_card".into(), "ExpiredCard".into()),
                ("card_declined".into(), "SuspectedFraud".into()),
                ("processing_error".into(), "IssuerUnavailable".into()),
                ("stolen_card".into(), "StolenCard".into()),
                ("lost_card".into(), "LostCard".into()),
                ("pickup_card".into(), "PickupCard".into()),
                ("generic_decline".into(), "CardDeclined".into()),
                ("rate_limit".into(), "RateLimitedByAcquirer".into()),
            ])),
        }
    }

    /// Build the Authorization header value.
    pub(super) fn auth_header(&self) -> String {
        format!("Bearer {}", self.secret_key)
    }

    /// Build form params for a PaymentIntent create call.
    pub(super) fn build_authorize_params(&self, req: &AuthorizeRequest) -> Vec<(String, String)> {
        let mut params: Vec<(String, String)> = vec![
            ("amount".into(), req.amount.amount_minor_units.to_string()),
            ("currency".into(), req.currency.to_lowercase()),
            ("payment_method".into(), req.payment_method_token.clone()),
            ("confirm".into(), "true".to_string()),
            ("capture_method".into(), "manual".to_string()),
        ];

        if !req.idempotency_key.is_empty() {
            params.push(("idempotency_key".into(), req.idempotency_key.clone()));
        }

        if let Some(metadata) = &req.metadata {
            if let Some(obj) = metadata.as_object() {
                for (key, val) in obj.iter() {
                    if let Some(s) = val.as_str() {
                        params.push((format!("metadata[{key}]"), s.to_string()));
                    }
                }
            }
        }

        params
    }

    /// Normalize a Stripe PaymentIntent response into our AuthorizeResponse.
    pub(super) fn normalize_authorize_response(
        &self,
        body: &Value,
        latency_ms: u32,
    ) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["id"].as_str().unwrap_or("unknown");
        let status = body["status"].as_str().unwrap_or("unknown");

        let authorize_status = match status {
            "succeeded" => AuthorizeStatus::Approved,
            "requires_action" => AuthorizeStatus::Requires3DS,
            "requires_payment_method" => AuthorizeStatus::Declined,
            "canceled" => AuthorizeStatus::Declined,
            _ => AuthorizeStatus::Declined,
        };

        let last_payment_error = body.get("last_payment_error");
        let decline_reason = last_payment_error
            .and_then(|e| e.get("decline_code"))
            .and_then(|c| c.as_str())
            .map(|code| self.decline_table.normalize(code));

        let three_ds_data = if status == "requires_action" {
            let next_action = body.get("next_action");
            next_action.and_then(|na| {
                let redirect = na.get("redirect_to_url")?;
                Some(ThreeDsData {
                    three_ds_version: "2.0".into(),
                    acs_url: redirect.get("url").and_then(|u| u.as_str()).map(String::from),
                    pareq: None,
                    md: None,
                    session_data: None,
                })
            })
        } else {
            None
        };

        Ok(AuthorizeResponse {
            status: authorize_status,
            acquirer_reference: Some(format!("pi_{}", id)),
            decline_reason,
            approved_amount: body
                .get("amount")
                .and_then(|a| a.as_i64())
                .map(|a| Money {
                    amount_minor_units: a,
                    currency: body["currency"].as_str().unwrap_or("usd").to_uppercase(),
                }),
            three_ds_data,
            latency_ms,
        })
    }
}

impl std::fmt::Debug for StripeConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeConnector")
            .field("environment", &self.environment)
            .field("base_url", &self.base_url)
            .finish()
    }
}
