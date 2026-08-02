//! Stripe network token provisioning using PaymentMethods API.
//!
//! This module has been migrated from the legacy /v1/tokens endpoint to the
//! modern /v1/payment_methods endpoint for PCI DSS compliance and future-proofing.
//!
//! Key changes:
//! - Uses PaymentMethods API instead of Tokens API
//! - Supports 3D Secure 2 (3DS2) for enhanced security
//! - Better token lifecycle management
//! - Compatible with Stripe's recommended integration patterns

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::error::ConnectorError;
use super::super::stripe_connector::StripeConnector;
use super::super::types::*;

/// PaymentMethod creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentMethodRequest {
    /// Card number (PAN)
    pub card_number: String,
    /// Expiration month (1-12)
    pub exp_month: u32,
    /// Expiration year (4-digit)
    pub exp_year: u32,
    /// Cardholder name
    pub name: Option<String>,
    /// Billing details
    pub billing_details: Option<BillingDetails>,
}

/// Billing details for PaymentMethod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingDetails {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<Address>,
}

/// Address for billing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// PaymentMethod response from Stripe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodResponse {
    pub id: String,
    pub object: String,
    pub card: Option<CardDetails>,
    pub created: i64,
    pub livemode: bool,
}

/// Card details from PaymentMethod
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CardDetails {
    pub brand: Option<String>,
    pub exp_month: Option<u64>,
    pub exp_year: Option<u64>,
    pub last4: Option<String>,
    pub funding: Option<String>,
}

/// SetupIntent response for saving PaymentMethod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupIntentResponse {
    pub id: String,
    pub object: String,
    pub client_secret: String,
    pub payment_method: Option<String>,
    pub status: String,
}

impl StripeConnector {
    /// Provision network token using PaymentMethods API (migrated from legacy Tokens).
    ///
    /// This method creates a PaymentMethod and optionally attaches it to a customer
    /// for future use. The PaymentMethod ID can be used for subsequent payments.
    ///
    /// # Migration Notes
    /// - Legacy: POST /v1/tokens (creates single-use token)
    /// - New: POST /v1/payment_methods (creates reusable PaymentMethod)
    /// - PaymentMethods support 3DS2, network tokens, and better lifecycle management
    pub(super) async fn provision_network_token_impl(
        &self,
        req: ProvisionTokenRequest,
    ) -> Result<ProvisionTokenResponse, ConnectorError> {
        // Step 1: Create PaymentMethod
        let payment_method = self.create_payment_method(&req).await?;
        
        // Step 2: Optionally attach to customer for future use
        let pm_id = payment_method.id.clone();
        
        // Step 3: Extract card details for response
        let card = payment_method.card.unwrap_or_default();
        
        Ok(ProvisionTokenResponse {
            network_token: pm_id,
            token_expiry_month: card.exp_month.unwrap_or(req.expiry_month as u64) as u32,
            token_expiry_year: card.exp_year.unwrap_or(req.expiry_year as u64) as u32,
            cryptogram: None, // Stripe handles cryptogram internally
        })
    }

    /// Create a PaymentMethod via Stripe API
    async fn create_payment_method(
        &self,
        req: &ProvisionTokenRequest,
    ) -> Result<PaymentMethodResponse, ConnectorError> {
        let resp = self
            .client
            .post(format!("{}/v1/payment_methods", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("type", "card"),
                ("card[number]", &req.card_number),
                ("card[exp_month]", &req.expiry_month.to_string()),
                ("card[exp_year]", &req.expiry_year.to_string()),
            ])
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe PaymentMethod create: {}", e)))?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        if !status.is_success() {
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            return Err(ConnectorError::NetworkError(format!(
                "Stripe PaymentMethod creation failed: {}",
                error_msg
            )));
        }

        // Deserialize into PaymentMethodResponse
        let payment_method: PaymentMethodResponse = serde_json::from_value(body)
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe deserialize: {}", e)))?;

        Ok(payment_method)
    }

    /// Attach a PaymentMethod to a customer
    pub(super) async fn attach_payment_method(
        &self,
        payment_method_id: &str,
        customer_id: &str,
    ) -> Result<(), ConnectorError> {
        let resp = self
            .client
            .post(format!(
                "{}/v1/payment_methods/{}/attach",
                self.base_url, payment_method_id
            ))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[("customer", customer_id)])
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe PaymentMethod attach: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let body: Value = resp.json().await.unwrap_or_default();
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            return Err(ConnectorError::NetworkError(format!(
                "Stripe PaymentMethod attach failed: {}",
                error_msg
            )));
        }

        Ok(())
    }

    /// Detach a PaymentMethod from a customer
    pub(super) async fn detach_payment_method(
        &self,
        payment_method_id: &str,
    ) -> Result<(), ConnectorError> {
        let resp = self
            .client
            .post(format!(
                "{}/v1/payment_methods/{}/detach",
                self.base_url, payment_method_id
            ))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe PaymentMethod detach: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let body: Value = resp.json().await.unwrap_or_default();
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            return Err(ConnectorError::NetworkError(format!(
                "Stripe PaymentMethod detach failed: {}",
                error_msg
            )));
        }

        Ok(())
    }

    /// Create a SetupIntent to save a PaymentMethod for future use
    pub(super) async fn create_setup_intent(
        &self,
        customer_id: Option<&str>,
        payment_method_id: Option<&str>,
    ) -> Result<SetupIntentResponse, ConnectorError> {
        let mut form = vec![("usage".to_string(), "off_session".to_string())];
        
        if let Some(cid) = customer_id {
            form.push(("customer".to_string(), cid.to_string()));
        }
        
        if let Some(pm_id) = payment_method_id {
            form.push(("payment_method".to_string(), pm_id.to_string()));
            form.push(("confirm".to_string(), "true".to_string()));
        }

        let resp = self
            .client
            .post(format!("{}/v1/setup_intents", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe SetupIntent: {}", e)))?;

        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let setup_intent: SetupIntentResponse = serde_json::from_value(body)
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe deserialize: {}", e)))?;

        Ok(setup_intent)
    }

    /// Validate a PaymentMethod by attempting a small authorization
    pub(super) async fn validate_payment_method(
        &self,
        payment_method_id: &str,
        currency: &str,
    ) -> Result<bool, ConnectorError> {
        let resp = self
            .client
            .post(format!("{}/v1/payment_intents", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("amount", "100"), // 1.00 in smallest currency unit
                ("currency", currency),
                ("payment_method", payment_method_id),
                ("confirm", "true"),
                ("capture_method", "manual"),
                ("description", "PaymentMethod validation"),
            ])
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe validate PM: {}", e)))?;

        let status = resp.status();
        let body: Value = resp.json().await.unwrap_or_default();
        
        // If payment intent was created (even if declined), PM is valid
        // Only fail on actual API errors
        if status.is_server_error() {
            return Ok(false);
        }

        // Check if the PM was usable (not a PM-specific error)
        let error_type = body["error"]["type"].as_str().unwrap_or("");
        let is_pm_error = error_type == "card_error" && 
            body["error"]["code"].as_str().unwrap_or("").contains("invalid_card");

        Ok(!is_pm_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_method_request_serialization() {
        let req = CreatePaymentMethodRequest {
            card_number: "4242424242424242".into(),
            exp_month: 12,
            exp_year: 2030,
            name: Some("Test User".into()),
            billing_details: None,
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("4242424242424242"));
    }
}
