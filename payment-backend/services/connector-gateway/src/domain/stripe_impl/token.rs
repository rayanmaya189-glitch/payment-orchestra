//! Stripe network token provisioning.

use serde_json::Value;

use super::super::error::ConnectorError;
use super::super::stripe_connector::StripeConnector;
use super::super::types::*;

impl StripeConnector {
    pub(super) async fn provision_network_token_impl(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
        // TODO: Migrate from /v1/tokens (legacy) to PaymentMethod-based tokenization.
        let resp = self
            .client
            .post(format!("{}/v1/tokens", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("card[number]", req.card_number),
                ("card[exp_month]", req.expiry_month.to_string()),
                ("card[exp_year]", req.expiry_year.to_string()),
                ("card[name]", req.cardholder_name.unwrap_or_default()),
            ])
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe token: {}", e)))?;

        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let token_id = body["id"].as_str().unwrap_or("tok_unknown");

        Ok(ProvisionTokenResponse {
            network_token: format!("tok_{}", token_id),
            token_expiry_month: body["card"]["exp_month"].as_u64().unwrap_or(req.expiry_month as u64) as u32,
            token_expiry_year: body["card"]["exp_year"].as_u64().unwrap_or(req.expiry_year as u64) as u32,
            cryptogram: None,
        })
    }
}
