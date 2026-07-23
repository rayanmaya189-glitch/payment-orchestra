//! gRPC service implementation for risk-service (BC-11).
//! Translates between protobuf types and domain types for fraud scoring.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::{RiskError, RiskLevel};
use crate::queries::QueryHandler;

use platform_proto::common::RiskAssessment as ProtoRiskAssessment;
use platform_proto::risk::risk_service_server::RiskService;
use platform_proto::risk::*;

pub struct RiskGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> RiskGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> RiskService for RiskGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn assess_risk(
        &self,
        request: Request<AssessRiskRequest>,
    ) -> Result<Response<AssessRiskResponse>, Status> {
        let req = request.into_inner();
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;

        let amount = req
            .amount
            .ok_or_else(|| Status::invalid_argument("amount is required"))?;

        // The proto has card_bin (first 6 digits), card_last_four (last 4),
        // payment_method_id, ip_address, customer_id, metadata_json.
        // The domain AssessRiskCommand needs billing_country and shipping_country
        // which we don't have in the proto — use reasonable defaults.
        let card_bin = if req.card_bin.len() >= 6 {
            req.card_bin[..6].to_string()
        } else {
            return Err(Status::invalid_argument("card_bin must be at least 6 digits"));
        };

        let cmd = commands::AssessRiskCommand {
            payment_intent_id,
            amount_minor_units: amount.amount_minor_units,
            currency: amount.currency_code,
            card_bin,
            billing_country: "AE".to_string(), // default since proto doesn't have this
            shipping_country: None,
            is_new_payment_method: false,       // default since proto doesn't have this
        };

        match self.commands.assess_risk(cmd).await {
            Ok(assessment) => {
                let requires_3ds = assessment.risk_level == RiskLevel::High
                    || assessment.risk_level == RiskLevel::Critical;
                let blocked = assessment.risk_level == RiskLevel::Critical;
                let recommendation = match assessment.risk_level {
                    RiskLevel::Low | RiskLevel::Medium => "approve",
                    RiskLevel::High => "review",
                    RiskLevel::Critical => "block",
                };

                Ok(Response::new(AssessRiskResponse {
                    assessment: Some(ProtoRiskAssessment {
                        risk_score: assessment.risk_score,
                        risk_level: assessment.risk_level.to_string(),
                        rule_version: assessment.rule_version,
                        risk_factors: assessment.risk_factors,
                    }),
                    requires_3ds,
                    blocked,
                    recommendation: recommendation.to_string(),
                }))
            }
            Err(e) => Err(risk_error_to_status(e)),
        }
    }

    async fn get_risk_profile(
        &self,
        request: Request<GetRiskProfileRequest>,
    ) -> Result<Response<RiskProfileView>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        // Get risk stats for the last 24 hours as a simple profile
        match self.queries.get_risk_stats(operator_id, 24).await {
            Ok(stats) => {
                // Get default rules for the active_rules list
                let default_rules = self.commands.get_default_rules().await;
                let active_rules: Vec<ActiveRule> = default_rules
                    .into_iter()
                    .map(|rule| ActiveRule {
                        rule_id: rule.rule_id,
                        name: rule.description,
                        severity: if rule.score_increment >= 0.4 {
                            "high".to_string()
                        } else if rule.score_increment >= 0.25 {
                            "medium".to_string()
                        } else {
                            "low".to_string()
                        },
                        enabled: rule.is_active,
                    })
                    .collect();

                Ok(Response::new(RiskProfileView {
                    operator_id: operator_id.to_string(),
                    overall_risk_score: stats.avg_risk_score,
                    total_transactions_scored: (stats.high_risk_count + stats.critical_risk_count) as i64,
                    high_risk_count: stats.high_risk_count as i64,
                    blocked_count: stats.critical_risk_count as i64,
                    active_rules,
                }))
            }
            Err(e) => Err(risk_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn risk_error_to_status(e: RiskError) -> Status {
    match e {
        RiskError::NotFound(id) => {
            Status::not_found(format!("Risk assessment not found for: {}", id))
        }
        RiskError::InvalidCardBin(bin) => {
            Status::invalid_argument(format!("Invalid card BIN: {}", bin))
        }
        RiskError::UnsupportedCurrency(currency) => {
            Status::invalid_argument(format!("Unsupported currency: {}", currency))
        }
    }
}

impl From<RiskError> for Status {
    fn from(e: RiskError) -> Self {
        risk_error_to_status(e)
    }
}
