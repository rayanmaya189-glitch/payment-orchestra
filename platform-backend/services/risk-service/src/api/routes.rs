use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::api::dto::{
    AssessRiskRequest, ErrorResponse, RiskAssessmentResponse, RiskFactorBreakdownResponse,
    RiskFactorResponse,
};
use crate::api::AppState;
use crate::application::commands::AssessPaymentRiskCommand;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/risk/assess", post(assess_risk))
        .route("/risk/assessments/{id}", get(get_assessment))
        .with_state(state)
}

/// POST /v1/risk/assess
async fn assess_risk(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<AssessRiskRequest>,
) -> Result<(StatusCode, Json<RiskAssessmentResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Derive operator_id from auth context instead of using request body
    let operator_id = shared_types::derive_operator_id(&auth.principal_id, &auth.role, Some(req.operator_id));

    let cmd = AssessPaymentRiskCommand {
        payment_intent_id: req.payment_intent_id,
        operator_id,
        amount_minor_units: req.amount_minor_units,
        currency: req.currency,
        ip_address: req.ip_address,
        user_agent: req.user_agent,
        country_code: req.country_code,
        merchant_country: req.merchant_country,
        is_whitelisted: req.is_whitelisted.unwrap_or(false),
        is_blacklisted: req.is_blacklisted.unwrap_or(false),
        recent_tx_count_from_ip: req.recent_tx_count_from_ip.unwrap_or(0),
        recent_tx_count_from_card: req.recent_tx_count_from_card.unwrap_or(0),
        principal_role: auth.role,
        principal_operator_id: operator_id,
    };

    match state.service.assess(cmd).await {
        Ok(assessment) => {
            let resp = RiskAssessmentResponse {
                assessment_id: assessment.assessment_id,
                payment_intent_id: assessment.payment_intent_id,
                score: assessment.score,
                decision: assessment.decision.as_str().to_string(),
                factors: assessment
                    .factors
                    .into_iter()
                    .map(|f| RiskFactorResponse {
                        rule_name: f.rule_name,
                        score: f.score,
                        weight: f.weight,
                        description: f.description,
                    })
                    .collect(),
                breakdown: RiskFactorBreakdownResponse {
                    amount_factor: assessment.breakdown.amount_factor,
                    velocity_factor: assessment.breakdown.velocity_factor,
                    geo_factor: assessment.breakdown.geo_factor,
                    blacklist_factor: assessment.breakdown.blacklist_factor,
                    whitelist_override: assessment.breakdown.whitelist_override,
                },
                created_at: assessment.created_at.to_rfc3339(),
            };
            Ok((StatusCode::OK, Json(resp)))
        }
        Err(e) => {
            let (status, code) = match &e {
                platform_error::PlatformError::AuthorizationDenied(_) => {
                    (StatusCode::FORBIDDEN, "AUTHORIZATION_DENIED")
                }
                platform_error::PlatformError::Validation(_) => {
                    (StatusCode::BAD_REQUEST, "VALIDATION_ERROR")
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            };
            Err((
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: code.to_string(),
                }),
            ))
        }
    }
}

/// GET /v1/risk/assessments/{id}
async fn get_assessment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RiskAssessmentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let aid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid assessment ID".to_string(),
                code: "INVALID_ID".to_string(),
            }),
        )
    })?;

    match state.service.get_assessment(aid).await {
        Ok(assessment) => {
            let resp = RiskAssessmentResponse {
                assessment_id: assessment.assessment_id,
                payment_intent_id: assessment.payment_intent_id,
                score: assessment.score,
                decision: assessment.decision.as_str().to_string(),
                factors: assessment
                    .factors
                    .into_iter()
                    .map(|f| RiskFactorResponse {
                        rule_name: f.rule_name,
                        score: f.score,
                        weight: f.weight,
                        description: f.description,
                    })
                    .collect(),
                breakdown: RiskFactorBreakdownResponse {
                    amount_factor: assessment.breakdown.amount_factor,
                    velocity_factor: assessment.breakdown.velocity_factor,
                    geo_factor: assessment.breakdown.geo_factor,
                    blacklist_factor: assessment.breakdown.blacklist_factor,
                    whitelist_override: assessment.breakdown.whitelist_override,
                },
                created_at: assessment.created_at.to_rfc3339(),
            };
            Ok(Json(resp))
        }
        Err(e) => {
            let status = match &e {
                platform_error::PlatformError::NotFound { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "NOT_FOUND".to_string(),
                }),
            ))
        }
    }
}
