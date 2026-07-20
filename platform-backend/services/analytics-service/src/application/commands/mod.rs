use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct GetPaymentAnalyticsCommand { pub operator_id: Uuid, pub start: String, pub end: String }
