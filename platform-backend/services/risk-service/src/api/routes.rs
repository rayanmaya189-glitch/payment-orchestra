use axum::{extract::{Path, State}, http::StatusCode, Json, Router};
use uuid::Uuid;
use super::dto::*;
use super::AppState;
use crate::application::services::{RiskService, RiskAssessmentResponse as ServiceResponse};

pub fn router(state: AppState) -> Router {
    Router::new().route("/risk/assess", axum::routing::post(assess_risk)).route("/risk/{id}", axum::routing::get(get_assessment)).with_state(state)
}

async fn assess_risk(State(state): State<AppState>, Json(req): Json<AssessRiskRequest>) -> Result<(StatusCode, Json<RiskAssessmentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::AssessRiskCommand { operator_id: Uuid::nil(), payment_intent_id: req.payment_intent_id, amount: req.amount };
    match state.service.assess_risk(cmd).await { Ok(r) => Ok((StatusCode::CREATED, Json(convert_response(r)))), Err(e) => Err(err(e)) }
}

async fn get_assessment(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<RiskAssessmentResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_assessment(id).await { Ok(r) => Ok(Json(convert_response(r))), Err(e) => Err(err(e)) }
}

fn convert_response(r: ServiceResponse) -> RiskAssessmentResponse {
    RiskAssessmentResponse { assessment_id: r.assessment_id, score: r.score, decision: r.decision, factors: r.factors.into_iter().map(|f| RiskFactorResponse { factor: f.factor, score: f.score, weight: f.weight, description: f.description }).collect() }
}
fn err(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) { let (s,c,m) = match &e { platform_error::PlatformError::NotFound{resource,id} => (StatusCode::NOT_FOUND,"NOT_FOUND",format!("{resource} {id}")), _ => (StatusCode::INTERNAL_SERVER_ERROR,"INTERNAL_ERROR","Internal error".into()) }; (s, Json(ErrorResponse{error:m,code:c.to_string()})) }
