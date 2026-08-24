//! Risk Service gRPC client — fraud scoring and risk assessment.
//!
//! Used by Orchestration Service to assess transaction risk
//! before routing to an acquirer.

use platform_proto::risk::risk_service_client::RiskServiceClient;
use platform_proto::risk::{AssessRiskRequest, AssessRiskResponse};

use crate::client::{ClientError, ServiceConnection};

/// Client for the Risk service (BC-11).
///
/// Provides synchronous risk scoring for the checkout hot path.
#[derive(Debug, Clone)]
pub struct RiskClient {
    conn: ServiceConnection,
}

impl RiskClient {
    /// Create a new Risk client connecting to the given address.
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("risk-service", addr).await?;
        Ok(Self { conn })
    }

    /// Create a lazy Risk client (connects on first RPC).
    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("risk-service", addr)?;
        Ok(Self { conn })
    }

    /// Assess the risk of a transaction before authorizing.
    #[allow(clippy::too_many_arguments)]
    pub async fn assess_risk(
        &self,
        payment_intent_id: String,
        operator_id: String,
        amount_minor_units: i64,
        currency_code: String,
        card_bin: String,
        card_last_four: String,
        payment_method_id: String,
        ip_address: String,
        customer_id: String,
        metadata_json: String,
    ) -> Result<AssessRiskResponse, ClientError> {
        let mut client = RiskServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(AssessRiskRequest {
            payment_intent_id,
            operator_id,
            amount: Some(platform_proto::common::Money {
                amount_minor_units,
                currency_code,
            }),
            card_bin,
            card_last_four,
            payment_method_id,
            ip_address,
            customer_id,
            metadata_json,
        });
        let response = client
            .assess_risk(request)
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "risk-service".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }
}
