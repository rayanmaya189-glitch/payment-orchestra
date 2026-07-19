use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OperatorMember {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}
