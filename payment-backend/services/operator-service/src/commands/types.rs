//! Command types for BC-01 Operator Management.

use uuid::Uuid;
use crate::domain::{Operator, OperatorStatus};

// ─── Command Types ──────────────────────────────────────────────────────────

pub struct RegisterOperator {
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub email: String,
}

pub struct VerifyEmail {
    pub operator_id: Uuid,
    pub verification_token: String,
}

pub struct UpdateOperatorStatus {
    pub operator_id: Uuid,
    pub new_status: OperatorStatus,
    pub reason: String,
    #[allow(dead_code)]
    pub changed_by: Uuid,
}

// ─── Command Results ────────────────────────────────────────────────────────

pub struct RegisterOperatorResult {
    pub operator: Operator,
    #[allow(dead_code)]
    pub verification_token: String,
}

pub struct VerifyEmailResult {
    pub operator: Operator,
}

pub struct UpdateOperatorStatusResult {
    pub operator: Operator,
}
