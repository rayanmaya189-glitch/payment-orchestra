use serde::{Deserialize, Serialize}; use uuid::Uuid; use shared_types::Money;
#[derive(Debug, Deserialize)]
pub struct OpenDisputeRequest { pub payment_intent_id: Uuid, pub reason: String, pub amount: Money }
#[derive(Debug, Deserialize)]
pub struct SubmitEvidenceRequest { pub evidence: String }
#[derive(Debug, Serialize)]
pub struct DisputeResponse { pub dispute_id: Uuid, pub status: String, pub reason: String, pub amount: i64, pub currency: String }
#[derive(Debug, Serialize)]
pub struct ErrorResponse { pub error: String, pub code: String }
