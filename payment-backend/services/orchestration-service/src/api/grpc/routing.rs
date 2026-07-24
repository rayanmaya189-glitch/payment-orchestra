use tonic::{Request, Response, Status};

use super::{OrchestrationGrpcService, parse_uuid, orchestration_error_to_status};
use crate::commands::{CommandHandler, ActivateRoutingPolicy};
use crate::domain;

use platform_proto::orchestration::orchestration_service_server::OrchestrationService;
use platform_proto::orchestration::*;
use platform_proto::common::Timestamp;

#[tonic::async_trait]
impl<C, Q> OrchestrationService for OrchestrationGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: crate::queries::QueryHandler + Send + Sync + 'static,
{
    async fn create_payment_intent(
        &self,
        _request: Request<CreatePaymentIntentRequest>,
    ) -> Result<Response<CreatePaymentIntentResponse>, Status> {
        unreachable!("implemented in payment module")
    }

    async fn authorize_payment_intent(
        &self,
        _request: Request<AuthorizePaymentIntentRequest>,
    ) -> Result<Response<AuthorizePaymentIntentResponse>, Status> {
        unreachable!("implemented in payment module")
    }

    async fn capture_payment_intent(
        &self,
        _request: Request<CapturePaymentIntentRequest>,
    ) -> Result<Response<CapturePaymentIntentResponse>, Status> {
        unreachable!("implemented in payment module")
    }

    async fn void_payment_intent(
        &self,
        _request: Request<VoidPaymentIntentRequest>,
    ) -> Result<Response<VoidPaymentIntentResponse>, Status> {
        unreachable!("implemented in payment module")
    }

    async fn refund_payment_intent(
        &self,
        _request: Request<RefundPaymentIntentRequest>,
    ) -> Result<Response<RefundPaymentIntentResponse>, Status> {
        unreachable!("implemented in payment module")
    }

    async fn get_payment_intent(
        &self,
        _request: Request<GetPaymentIntentRequest>,
    ) -> Result<Response<PaymentIntentView>, Status> {
        unreachable!("implemented in payment module")
    }

    async fn activate_routing_policy(
        &self,
        request: Request<ActivateRoutingPolicyRequest>,
    ) -> Result<Response<ActivateRoutingPolicyResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;
        let rules: Vec<domain::RoutingRule> = req.rules.into_iter().map(|r| {
            let link_id = parse_uuid(&r.acquirer_link_id, "acquirer_link_id").unwrap_or_default();
            domain::RoutingRule {
                acquirer_link_id: link_id,
                priority: r.priority as i32,
                condition: domain::RoutingCondition::all(),
            }
        }).collect();

        let failover_config = req.failover_config.map(|f| domain::FailoverConfig {
            max_hops: f.max_hops as u8,
            latency_budget_ms: f.latency_budget_ms,
        }).unwrap_or_default();

        let cmd = ActivateRoutingPolicy {
            operator_id,
            rules,
            failover_config,
            partial_auth_strategy: domain::PartialAuthStrategy::AcceptPartial,
            rotation_strategy: domain::RotationStrategy::Priority,
            max_transaction_amount_minor: None,
        };

        match self.commands.activate_routing_policy(cmd).await {
            Ok(result) => {
                Ok(Response::new(ActivateRoutingPolicyResponse {
                    routing_policy_id: result.routing_policy_id.to_string(),
                    version: result.version,
                    status: result.status.to_string(),
                }))
            }
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }

    async fn get_routing_policy(
        &self,
        request: Request<GetRoutingPolicyRequest>,
    ) -> Result<Response<RoutingPolicyView>, Status> {
        let req = request.into_inner();
        let policy_id = parse_uuid(&req.policy_id, "policy_id")?;

        match self.queries.get_active_routing_policy(crate::queries::GetActiveRoutingPolicyQuery { operator_id: policy_id }).await {
            Ok(Some(policy)) => {
                let proto_rules: Vec<RoutingRule> = policy.rules.iter().map(|r| RoutingRule {
                    acquirer_link_id: r.acquirer_link_id.to_string(),
                    priority: r.priority as u32,
                    condition_json: String::new(),
                    rotation_strategy: "priority".into(),
                }).collect();

                Ok(Response::new(RoutingPolicyView {
                    policy_id: policy.routing_policy_id.to_string(),
                    operator_id: policy.operator_id.to_string(),
                    rules: proto_rules,
                    failover_config: Some(platform_proto::orchestration::FailoverConfig {
                        max_hops: policy.failover_config.max_hops as u32,
                        latency_budget_ms: policy.failover_config.latency_budget_ms,
                        retryable_decline_codes: vec![],
                        retry_unknown_as_fallback: false,
                    }),
                    version: policy.version,
                    status: policy.status.to_string(),
                    created_at: Some(Timestamp { unix_ms: policy.created_at.timestamp_millis() }),
                }))
            }
            Ok(None) => Err(Status::not_found("Routing policy not found")),
            Err(e) => Err(orchestration_error_to_status(e)),
        }
    }
}
