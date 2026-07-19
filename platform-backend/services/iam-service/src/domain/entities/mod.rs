use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RoleAssignment {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub role_name: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct MfaEnrollment {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub method: String,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
    pub enrolled_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct BackupCode {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub code_hash: Vec<u8>,
    pub used: bool,
    pub used_at: Option<DateTime<Utc>>,
}
