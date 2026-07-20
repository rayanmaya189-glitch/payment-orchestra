use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateSubscriptionCommand {
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub interval: String,
    pub interval_count: Option<i32>,
    pub trial_period_days: Option<i32>,
    pub payment_method_token_id: Option<String>,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct CancelSubscriptionCommand {
    pub subscription_id: Uuid,
    pub reason: String,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct ChargeSubscriptionCommand {
    pub subscription_id: Uuid,
}
