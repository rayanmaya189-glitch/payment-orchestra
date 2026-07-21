use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request body for POST /v1/risk/assess.
#[derive(Debug, Deserialize)]
pub struct AssessRiskRequest {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub country_code: Option<String>,
    pub merchant_country: Option<String>,
    pub is_whitelisted: Option<bool>,
    pub is_blacklisted: Option<bool>,
    pub recent_tx_count_from_ip: Option<u32>,
    pub recent_tx_count_from_card: Option<u32>,
}

/// Response body for a successful risk assessment.
#[derive(Debug, Serialize)]
pub struct RiskAssessmentResponse {
    pub assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub score: f64,
    pub decision: String,
    pub factors: Vec<RiskFactorResponse>,
    pub breakdown: RiskFactorBreakdownResponse,
    pub created_at: String,
}

/// A single risk factor in the response.
#[derive(Debug, Serialize)]
pub struct RiskFactorResponse {
    pub rule_name: String,
    pub score: f64,
    pub weight: f64,
    pub description: String,
}

/// Per-category breakdown in the response.
#[derive(Debug, Serialize)]
pub struct RiskFactorBreakdownResponse {
    pub amount_factor: f64,
    pub velocity_factor: f64,
    pub geo_factor: f64,
    pub blacklist_factor: f64,
    pub whitelist_override: bool,
}

/// Standard error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}
