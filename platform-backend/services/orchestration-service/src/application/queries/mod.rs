use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GetPaymentIntentQuery {
    pub payment_intent_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ListPaymentIntentsQuery {
    pub operator_id: Uuid,
    pub status: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}
