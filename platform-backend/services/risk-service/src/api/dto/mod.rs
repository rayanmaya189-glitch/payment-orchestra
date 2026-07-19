use serde::{Deserialize, Serialize}; use uuid::Uuid; use shared_types::Money;
#[derive(Debug, Deserialize)]
pub struct AssessRiskRequest { pub payment_intent_id: Uuid, pub amount: Money }
#[derive(Debug, Serialize)]
pub struct RiskAssessmentResponse { pub assessment_id: Uuid, pub score: f64, pub decision: String, pub factors: Vec<RiskFactorResponse> }
#[derive(Debug, Serialize)]
pub struct RiskFactorResponse { pub factor: String, pub score: f64, pub weight: f64, pub description: String }
#[derive(Debug, Serialize)]
pub struct ErrorResponse { pub error: String, pub code: String }
