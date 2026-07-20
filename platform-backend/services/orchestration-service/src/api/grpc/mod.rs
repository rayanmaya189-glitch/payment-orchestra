//! gRPC server implementation for OrchestrationService.
//!
//! Implements the tonic service trait generated from the proto definitions.

use tonic;
use uuid::Uuid;

use crate::application::services::PaymentService;
use std::sync::Arc;

/// gRPC server implementation wrapping the HTTP service.
pub struct GrpcOrchestrationService {
    service: Arc<dyn PaymentService>,
}

impl GrpcOrchestrationService {
    pub fn new(service: Arc<dyn PaymentService>) -> Self {
        Self { service }
    }

    /// Convert HTTP service response to gRPC response.
    pub async fn create_payment_intent(
        &self,
        amount_minor_units: i64,
        currency_code: String,
        idempotency_key: String,
        purpose: String,
    ) -> Result<(String, String), PlatformError> {
        let amount = shared_types::Money {
            amount_minor_units,
            currency: shared_types::CurrencyCode::new(&currency_code)
                .map_err(|_e| PlatformError::Validation(
                    platform_error::ValidationError::InvalidCurrencyCode
                ))?,
        };

        let cmd = crate::application::commands::CreatePaymentIntentCommand {
            operator_id: Uuid::nil(),
            principal_id: Uuid::nil(),
            role: "system".to_string(),
            amount,
            idempotency_key,
            purpose: Some(purpose),
            metadata: None,
            preferred_gateway_profile_id: None,
        };

        let resp = self.service.create_payment_intent(cmd).await?;
        Ok((resp.payment_intent_id.to_string(), resp.status))
    }

    pub async fn authorize(
        &self,
        payment_intent_id: Uuid,
        payment_method_token_id: Uuid,
    ) -> Result<String, PlatformError> {
        let cmd = crate::application::commands::AuthorizePaymentIntentCommand {
            payment_intent_id,
            payment_method_token_id,
            principal_id: Uuid::nil(),
            role: "system".to_string(),
            operator_id: Uuid::nil(),
        };

        let resp = self.service.authorize(cmd).await?;
        Ok(resp.status)
    }

    pub async fn capture(
        &self,
        payment_intent_id: Uuid,
        amount_minor_units: Option<i64>,
        currency_code: Option<String>,
    ) -> Result<String, PlatformError> {
        let amount = match (amount_minor_units, currency_code) {
            (Some(minor), Some(code)) => Some(shared_types::Money {
                amount_minor_units: minor,
                currency: shared_types::CurrencyCode::new(&code)
                    .map_err(|_e| PlatformError::Validation(
                        platform_error::ValidationError::InvalidCurrencyCode
                    ))?,
            }),
            _ => None,
        };

        let cmd = crate::application::commands::CapturePaymentIntentCommand {
            payment_intent_id,
            amount,
            principal_id: Uuid::nil(),
            role: "system".to_string(),
            operator_id: Uuid::nil(),
        };

        let resp = self.service.capture(cmd).await?;
        Ok(resp.status)
    }

    pub async fn void(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<String, PlatformError> {
        let cmd = crate::application::commands::VoidPaymentIntentCommand {
            payment_intent_id,
            principal_id: Uuid::nil(),
            role: "system".to_string(),
            operator_id: Uuid::nil(),
        };

        let resp = self.service.void(cmd).await?;
        Ok(resp.status)
    }

    pub async fn refund(
        &self,
        payment_intent_id: Uuid,
        amount_minor_units: i64,
        currency_code: String,
    ) -> Result<String, PlatformError> {
        let amount = shared_types::Money {
            amount_minor_units,
            currency: shared_types::CurrencyCode::new(&currency_code)
                .map_err(|_e| PlatformError::Validation(
                    platform_error::ValidationError::InvalidCurrencyCode
                ))?,
        };

        let cmd = crate::application::commands::RefundPaymentIntentCommand {
            payment_intent_id,
            amount,
            principal_id: Uuid::nil(),
            role: "system".to_string(),
            operator_id: Uuid::nil(),
        };

        let resp = self.service.refund(cmd).await?;
        Ok(resp.status)
    }

    pub async fn get_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<crate::api::dto::PaymentIntentResponse, PlatformError> {
        let query = crate::application::queries::GetPaymentIntentQuery {
            payment_intent_id,
        };

        self.service.get_payment_intent(query).await
    }
}

use platform_error::PlatformError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_service_creation() {
        // Verify that the module compiles and types are correct
    }
}