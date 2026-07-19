use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ComplianceOfficer {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub operator_id: Uuid,
    pub role: String,
    pub assigned_at: DateTime<Utc>,
}
