//! Orchestration Service gRPC client — payment intent lifecycle.
//!
//! Used by API Gateway to create, authorize, capture, refund,
//! and void payment intents.

use platform_proto::orchestration::orchestration_service_client::OrchestrationServiceClient;
use platform_proto::orchestration::{
    CreatePaymentIntentRequest, CreatePaymentIntentResponse,
    AuthorizePaymentIntentRequest, AuthorizePaymentIntentResponse,
    CapturePaymentIntentRequest, CapturePaymentIntentResponse,
    RefundPaymentIntentRequest, RefundPaymentIntentResponse,
    VoidPaymentIntentRequest, VoidPaymentIntentResponse,
    GetPaymentIntentRequest, PaymentIntentView,
};

use crate::client::{ClientError, ServiceConnection};

/// Client for the Orchestration Service (BC-05).
#[derive(Debug, Clone)]
pub struct OrchestrationClient {
    conn: ServiceConnection,
}

impl OrchestrationClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("orchestration-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("orchestration-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn create_payment_intent(
        &self,
        request: CreatePaymentIntentRequest,
    ) -> Result<CreatePaymentIntentResponse, ClientError> {
        let mut client = OrchestrationServiceClient::new(self.conn.channel().clone());
        let response = client
            .create_payment_intent(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed { service: "orchestration-service".into(), source: e })?;
        Ok(response.into_inner())
    }

    pub async fn authorize(
        &self,
        request: AuthorizePaymentIntentRequest,
    ) -> Result<AuthorizePaymentIntentResponse, ClientError> {
        let mut client = OrchestrationServiceClient::new(self.conn.channel().clone());
        let response = client
            .authorize_payment_intent(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed { service: "orchestration-service".into(), source: e })?;
        Ok(response.into_inner())
    }

    pub async fn capture(
        &self,
        request: CapturePaymentIntentRequest,
    ) -> Result<CapturePaymentIntentResponse, ClientError> {
        let mut client = OrchestrationServiceClient::new(self.conn.channel().clone());
        let response = client
            .capture_payment_intent(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed { service: "orchestration-service".into(), source: e })?;
        Ok(response.into_inner())
    }

    pub async fn refund(
        &self,
        request: RefundPaymentIntentRequest,
    ) -> Result<RefundPaymentIntentResponse, ClientError> {
        let mut client = OrchestrationServiceClient::new(self.conn.channel().clone());
        let response = client
            .refund_payment_intent(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed { service: "orchestration-service".into(), source: e })?;
        Ok(response.into_inner())
    }

    pub async fn void(
        &self,
        request: VoidPaymentIntentRequest,
    ) -> Result<VoidPaymentIntentResponse, ClientError> {
        let mut client = OrchestrationServiceClient::new(self.conn.channel().clone());
        let response = client
            .void_payment_intent(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed { service: "orchestration-service".into(), source: e })?;
        Ok(response.into_inner())
    }

    /// Get payment intent by ID — returns PaymentIntentView directly.
    pub async fn get_payment_intent(
        &self,
        payment_intent_id: String,
    ) -> Result<PaymentIntentView, ClientError> {
        let mut client = OrchestrationServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(GetPaymentIntentRequest { payment_intent_id });
        let response = client
            .get_payment_intent(request)
            .await
            .map_err(|e| ClientError::RpcFailed { service: "orchestration-service".into(), source: e })?;
        Ok(response.into_inner())
    }
}
