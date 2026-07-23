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
        // which we don't have in the proto — try to extract from metadata_json
        // or use card BIN prefix as a best-effort guess.
        let card_bin = if req.card_bin.len() >= 6 {
            req.card_bin[..6].to_string()
        } else {
            return Err(Status::invalid_argument("card_bin must be at least 6 digits"));
        };

        // Try to extract billing_country from metadata_json if available
        let billing_country = if !req.metadata_json.is_empty() {
            serde_json::from_str::<std::collections::HashMap<String, String>>(&req.metadata_json)
                .ok()
                .and_then(|m| m.get("billing_country").cloned())
                .unwrap_or_else(|| default_country_for_bin(&card_bin))
        } else {
            default_country_for_bin(&card_bin)
        };

        // Try to extract shipping_country from metadata_json
        let shipping_country = if !req.metadata_json.is_empty() {
            serde_json::from_str::<std::collections::HashMap<String, String>>(&req.metadata_json)
                .ok()
                .and_then(|m| m.get("shipping_country").cloned())
        } else {
            None
        };

        let cmd = commands::AssessRiskCommand {
            payment_intent_id,
            amount_minor_units: amount.amount_minor_units,
            currency: amount.currency_code,
            card_bin,
            billing_country,
            shipping_country,
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

/// Best-effort country resolution from card BIN (first 6 digits).
/// Based on known BIN ranges for common acquirers in the MENA region.
/// Returns "AE" (UAE) as default since the platform primarily targets the UAE market.
fn default_country_for_bin(bin: &str) -> String {
    // Known BIN ranges: 4xxxxx = Visa, 5xxxxx = Mastercard
    // For Phase 1, extract country from BIN issuer prefix if possible
    let prefix = &bin[..4];
    match prefix {
        // UAE-issued BIN ranges (Visa/Mastercard UAE prefixes)
        "4001" | "4017" | "4038" | "4069" | "4120" | "4179" | "4194"
        | "4225" | "4264" | "4283" | "4310" | "4347" | "4383" | "4423"
        | "4443" | "4464" | "4525" | "4541" | "4553" | "4567" | "4622"
        | "4703" | "4718" | "4743" | "4767" | "4903" | "4921" | "4941"
        | "5122" | "5185" | "5212" | "5246" | "5291" | "5312" | "5373"
        | "5414" | "5469" | "5542" | "5578" | "5591" => "AE".to_string(),
        // Saudi Arabia
        "4135" | "4412" | "4469" | "4555" | "4834" | "4855" | "4901"
        | "5167" | "5229" | "5325" | "5375" | "5435" | "5510" | "5584" => "SA".to_string(),
        // Kuwait
        "4066" | "4200" | "4248" | "4317" | "4385" | "4515" | "4526"
        | "4554" | "4590" | "4770" | "4894" | "4902" | "5141" | "5306"
        | "5313" | "5433" | "5482" | "5523" => "KW".to_string(),
        // Qatar
        "4024" | "4102" | "4157" | "4285" | "4351" | "4452" | "4515"
        | "4614" | "4762" | "4866" | "4905" | "5130" | "5159" | "5316"
        | "5379" | "5428" | "5487" | "5548" => "QA".to_string(),
        // Bahrain
        "4004" | "4096" | "4156" | "4219" | "4301" | "4374" | "4461"
        | "4562" | "4674" | "4768" | "4831" | "4922" | "5122" | "5211"
        | "5272" | "5315" | "5397" | "5464" | "5532" => "BH".to_string(),
        _ => "AE".to_string(), // Default to UAE
    }
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
