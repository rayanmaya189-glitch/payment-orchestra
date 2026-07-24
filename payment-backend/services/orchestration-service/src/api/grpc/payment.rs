use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::{OrchestrationGrpcService, parse_uuid, orchestration_error_to_status};
use crate::commands::{CommandHandler, CreatePaymentIntent, AuthorizePaymentIntent, CapturePaymentIntent, VoidPaymentIntent, RefundPaymentIntent};
use crate::domain::{self, PaymentStatus, SourceType};

use platform_proto::orchestration::orchestration_service_server::OrchestrationService;
use platform_proto::orchestration::*;
use platform_proto::common::{Money as ProtoMoney, Timestamp};

#[tonic::async_trait]
impl<C, Q> OrchestrationService for OrchestrationGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: crate::queries::QueryHandler + Send + Sync + 'static,
{
    async fn create_payment_intent(
        &self,
        request: Request<CreatePaymentIntentRequest>,
    ) -> Result<Response<CreatePaymentIntentResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;
        let amount = req.amount.ok_or_else(|| Status::invalid_argument("amount is required"))?;
        let purpose = match req.purpose.as_str() {
            "card_verification" => domain::PaymentPurpose::CardVerification,
            _ => domain::PaymentPurpose::Payment,
        };
        let source_type = match req.source_type.as_str() {
            "merchant_api" => SourceType::MerchantApi,
            "invoice" => SourceType::Invoice,
            "subscription" => SourceType::Subscription,
            "payment_link" => SourceType::PaymentLink,
            "ai_assistant" => SourceType::AiAssistant,
            _ => SourceType::System,
        };

        let cmd = CreatePaymentIntent {
            operator_id,
            idempotency_key: req.idempotency_key,
            amount_minor_units: amount.amount_minor_units,
            currency: amount.currency_code,
            purpose,
            metadata: if req.metadata_json.is_empty() { None } else { serde_json::from_str(&req.metadata_json).ok() },
            source_type,
            source_id: None,
            payment_method_token_id: None,
            preferred_gateway_profile_id: None,
        };

        match self.commands.create_payment_intent(cmd).await {
            Ok(result) => {
                Ok(Response::new(CreatePaymentIntentResponse {
                    payment_intent_id: result.payment_intent_id.to_string(),
                    status: result.status.to_string(),
                    created_at: Some(Timestamp { unix_ms: chrono::Utc::now().timestamp_millis() }),
                }))
            }
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }

    async fn authorize_payment_intent(
        &self,
        request: Request<AuthorizePaymentIntentRequest>,
    ) -> Result<Response<AuthorizePaymentIntentResponse>, Status> {
        let req = request.into_inner();
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;
        let payment_method_token_id = parse_uuid(&req.payment_method_token_id, "payment_method_token_id")?;

        let cmd = AuthorizePaymentIntent {
            payment_intent_id,
            payment_method_token_id,
            card_scheme: "visa".into(),
            actor_id: Uuid::nil(),
        };

        match self.commands.authorize_payment_intent(cmd).await {
            Ok(result) => {
                let attempts: Vec<RoutingAttemptResult> = result.routing_attempts.iter().map(|a| {
                    RoutingAttemptResult {
                        acquirer_connector_id: a.connector_id.clone(),
                        approved: matches!(a.status, domain::AttemptStatus::Approved),
                        normalized_decline_reason: a.decline_reason.as_ref().map(|d| d.to_string()).unwrap_or_default(),
                        latency_ms: a.latency_ms,
                        acquirer_reference: a.acquirer_reference.clone().unwrap_or_default(),
                    }
                }).collect();

                let final_decline = if result.status.is_failed() {
                    result.routing_attempts.last()
                        .and_then(|a| a.decline_reason.as_ref().map(|d| d.to_string()))
                        .unwrap_or_else(|| "All routes declined".into())
                } else {
                    String::new()
                };

                Ok(Response::new(AuthorizePaymentIntentResponse {
                    status: result.status.to_string(),
                    attempts,
                    final_decline_reason: final_decline,
                }))
            }
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }

    async fn capture_payment_intent(
        &self,
        request: Request<CapturePaymentIntentRequest>,
    ) -> Result<Response<CapturePaymentIntentResponse>, Status> {
        let req = request.into_inner();
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;
        let amount_minor = req.amount.map(|a| a.amount_minor_units);

        let cmd = CapturePaymentIntent {
            payment_intent_id,
            amount_minor_units: amount_minor,
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::nil(),
        };

        match self.commands.capture_payment_intent(cmd).await {
            Ok(result) => {
                let _is_partial = matches!(result.status, PaymentStatus::PartiallyCaptured);
                Ok(Response::new(CapturePaymentIntentResponse {
                    status: result.status.to_string(),
                    captured_amount: Some(ProtoMoney {
                        amount_minor_units: result.captured_amount.amount_minor_units,
                        currency_code: result.captured_amount.currency.clone(),
                    }),
                }))
            }
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }

    async fn void_payment_intent(
        &self,
        request: Request<VoidPaymentIntentRequest>,
    ) -> Result<Response<VoidPaymentIntentResponse>, Status> {
        let req = request.into_inner();
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;

        let cmd = VoidPaymentIntent {
            payment_intent_id,
            actor_id: Uuid::nil(),
        };

        match self.commands.void_payment_intent(cmd).await {
            Ok(_) => {
                Ok(Response::new(VoidPaymentIntentResponse {
                    status: "voided".into(),
                }))
            }
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }

    async fn refund_payment_intent(
        &self,
        request: Request<RefundPaymentIntentRequest>,
    ) -> Result<Response<RefundPaymentIntentResponse>, Status> {
        let req = request.into_inner();
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;
        let amount_minor = req.amount.map(|a| a.amount_minor_units).unwrap_or(0);

        let cmd = RefundPaymentIntent {
            payment_intent_id,
            amount_minor_units: amount_minor,
            actor_id: Uuid::nil(),
        };

        match self.commands.refund_payment_intent(cmd).await {
            Ok(result) => {
                let _is_partial = matches!(result.status, PaymentStatus::PartiallyRefunded);
                Ok(Response::new(RefundPaymentIntentResponse {
                    status: result.status.to_string(),
                    refund_id: format!("ref_{}", Uuid::now_v7()),
                    refunded_amount: Some(ProtoMoney {
                        amount_minor_units: result.refunded_amount.amount_minor_units,
                        currency_code: result.refunded_amount.currency.clone(),
                    }),
                }))
            }
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }

    async fn get_payment_intent(
        &self,
        request: Request<GetPaymentIntentRequest>,
    ) -> Result<Response<PaymentIntentView>, Status> {
        let req = request.into_inner();
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;

        match self.queries.get_payment_intent(crate::queries::GetPaymentIntentQuery { payment_intent_id }).await {
            Ok(Some(pi)) => {
                Ok(Response::new(PaymentIntentView {
                    payment_intent_id: pi.payment_intent_id.to_string(),
                    operator_id: pi.operator_id.to_string(),
                    status: pi.status.to_string(),
                    requested_amount: Some(ProtoMoney {
                        amount_minor_units: pi.requested_amount.amount_minor_units,
                        currency_code: pi.requested_amount.currency.clone(),
                    }),
                    authorized_amount: Some(ProtoMoney {
                        amount_minor_units: pi.authorized_amount.amount_minor_units,
                        currency_code: pi.authorized_amount.currency.clone(),
                    }),
                    captured_amount: Some(ProtoMoney {
                        amount_minor_units: pi.captured_amount.amount_minor_units,
                        currency_code: pi.captured_amount.currency.clone(),
                    }),
                    refunded_amount: Some(ProtoMoney {
                        amount_minor_units: pi.refunded_amount.amount_minor_units,
                        currency_code: pi.refunded_amount.currency.clone(),
                    }),
                    currency: pi.currency.clone(),
                    created_at: Some(Timestamp { unix_ms: pi.created_at.timestamp_millis() }),
                    updated_at: Some(Timestamp { unix_ms: pi.updated_at.timestamp_millis() }),
                    source_type: pi.source_type.unwrap_or_default(),
                    source_id: pi.source_id.map(|id| id.to_string()).unwrap_or_default(),
                }))
            }
            Ok(None) => Err(Status::not_found("PaymentIntent not found")),
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }

    async fn activate_routing_policy(
        &self,
        _request: Request<ActivateRoutingPolicyRequest>,
    ) -> Result<Response<ActivateRoutingPolicyResponse>, Status> {
        unreachable!("implemented in routing module")
    }

    async fn get_routing_policy(
        &self,
        _request: Request<GetRoutingPolicyRequest>,
    ) -> Result<Response<RoutingPolicyView>, Status> {
        unreachable!("implemented in routing module")
    }
}
