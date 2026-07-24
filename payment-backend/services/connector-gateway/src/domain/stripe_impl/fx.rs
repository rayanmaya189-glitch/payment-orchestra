//! Stripe FX rate, settlement cycle, and 3DS operations.

use chrono::Utc;
use serde_json::Value;

use super::super::error::ConnectorError;
use super::super::stripe_connector::StripeConnector;
use super::super::types::*;

const STRIPE_API_VERSION: &str = "2025-02-24.acacia";

impl StripeConnector {
    pub(super) async fn get_fx_rate_impl(&self, req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
        let resp = self
            .client
            .post(format!("{}/v1/currencies/conversion_rates", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("source_currency", req.source_currency.to_lowercase()),
                ("target_currencies[]", req.target_currency.to_lowercase()),
            ])
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe FX: {}", e)))?;

        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let rate = body["rates"][req.target_currency.to_lowercase()]
            .as_str()
            .unwrap_or("1.0")
            .to_string();

        let rate_parsed: f64 = rate.parse().unwrap_or(1.0);
        let rate_minor = (rate_parsed * 1_000_000.0) as i64;
        let converted = (req.amount.amount_minor_units as f64 * rate_parsed) as i64;
        let fee = (converted as f64 * 0.01) as i64;

        Ok(FxRateResponse {
            rate,
            rate_minor_units: rate_minor,
            converted_amount: Money {
                amount_minor_units: converted,
                currency: req.target_currency.to_uppercase(),
            },
            fee: Some(Money {
                amount_minor_units: fee,
                currency: req.target_currency.to_uppercase(),
            }),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        })
    }

    pub(super) async fn check_3ds_enrollment_impl(&self, req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
        let resp = self
            .client
            .post(format!("{}/v1/payment_intents", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Stripe-Version", STRIPE_API_VERSION)
            .form(&[
                ("amount", req.amount.amount_minor_units.to_string()),
                ("currency", req.currency.to_lowercase()),
                ("payment_method_data[type]", "card".to_string()),
                ("payment_method_data[card][number]", req.card_number.clone()),
                ("payment_method_data[card][exp_month]", "12".to_string()),
                ("payment_method_data[card][exp_year]", "2030".to_string()),
                ("confirm", "false".to_string()),
            ])
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe 3DS check: {}", e)))?;

        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let requires_3ds = body["status"] == "requires_action";

        Ok(Check3dsResponse {
            requires_3ds,
            three_ds_data: if requires_3ds {
                Some(ThreeDsData {
                    three_ds_version: "2.0".into(),
                    acs_url: body["next_action"]["redirect_to_url"]["url"]
                        .as_str()
                        .map(String::from),
                    pareq: None,
                    md: None,
                    session_data: None,
                })
            } else {
                None
            },
        })
    }

    pub(super) fn authenticate_3ds_impl(&self, req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
        Ok(Authenticate3dsResponse {
            authenticated: req.authentication_value.is_some(),
            three_ds_status: req
                .authentication_value
                .map(|_| "authenticated".into())
                .unwrap_or_else(|| "failed".into()),
            eci: req.three_ds_data.session_data,
        })
    }
}
