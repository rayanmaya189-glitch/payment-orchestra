use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AssessPaymentRiskCommand {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
