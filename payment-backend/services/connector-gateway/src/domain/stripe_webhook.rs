//! Stripe webhook signature verification and event parsing.

use std::collections::HashMap;

use ring::hmac;
use serde_json::Value;

use super::error::ConnectorError;
use super::stripe_connector::StripeConnector;
use super::types::ConnectorEvent;

impl StripeConnector {
    /// Verify Stripe webhook signature using HMAC-SHA256.
    pub fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        let signature_header = headers
            .get("stripe-signature")
            .or_else(|| headers.get("Stripe-Signature"))
            .or_else(|| {
                headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("stripe-signature"))
                    .map(|(_, v)| v)
            })
            .ok_or(ConnectorError::InvalidSignature)?;

        if self.webhook_secret.is_empty() {
            return Err(ConnectorError::InvalidSignature);
        }

        // Parse the signature header: t=timestamp,v1=signature
        let parts: Vec<&str> = signature_header.split(',').collect();
        let mut timestamp = None;
        let mut signature = None;

        for part in &parts {
            if let Some(t_val) = part.strip_prefix("t=") {
                timestamp = Some(t_val.to_string());
            } else if let Some(sig_val) = part.strip_prefix("v1=") {
                signature = Some(sig_val.to_string());
            }
        }

        let _timestamp = timestamp.ok_or(ConnectorError::InvalidSignature)?;
        let signature = signature.ok_or(ConnectorError::InvalidSignature)?;

        // Build the expected signature payload: timestamp.body
        let signed_payload = format!("{}.{}", _timestamp, String::from_utf8_lossy(body));

        // Compute HMAC-SHA256 using the webhook secret
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let computed_sig = hmac::sign(&key, signed_payload.as_bytes());
        let computed_hex = hex::encode(computed_sig.as_ref());

        // Constant-time comparison
        if computed_hex == signature {
            Ok(())
        } else {
            Err(ConnectorError::InvalidSignature)
        }
    }

    /// Parse a Stripe webhook body into a ConnectorEvent.
    pub fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let parsed: Value = serde_json::from_slice(body)
            .map_err(|e| ConnectorError::InvalidRequest(format!("Invalid webhook JSON: {}", e)))?;

        let event_type = parsed["type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(ConnectorEvent {
            event_type,
            payload: parsed,
        })
    }
}
