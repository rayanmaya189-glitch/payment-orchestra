use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OpenDisputeCommand {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub reason: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub acquirer_reference: String,
    pub connector_id: String,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct SubmitEvidenceCommand {
    pub dispute_id: Uuid,
    pub evidence: serde_json::Value,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct ResolveDisputeCommand {
    pub dispute_id: Uuid,
    pub decision: String,
    pub reason: String,
    pub principal_id: Uuid,
    pub role: String,
}
