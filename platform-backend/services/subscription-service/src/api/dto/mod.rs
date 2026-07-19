use serde::{Deserialize, Serialize}; use uuid::Uuid; use shared_types::Money;
#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest { pub customer_id: Uuid, pub amount: Money, pub interval: String }
#[derive(Debug, Serialize)]
pub struct SubscriptionResponse { pub subscription_id: Uuid, pub status: String, pub amount: i64, pub currency: String, pub interval: String, pub current_period_end: String }
#[derive(Debug, Serialize)]
pub struct ErrorResponse { pub error: String, pub code: String }
