//! Command types for BC-03 Merchant Compliance.

use uuid::Uuid;
use crate::domain::{KybCase, AmlAlert};

// ─── Command Types ──────────────────────────────────────────────────────────

pub struct SubmitKybEvidence {
    pub operator_id: Uuid,
    pub document_ids: Vec<Uuid>,
    pub submitted_by: Uuid,
}

pub struct ReviewKybCase {
    pub kyb_case_id: Uuid,
    pub approved: bool,
    pub reason: Option<String>,
    pub _reviewed_by: Uuid,
}

pub struct ScanTransaction {
    pub transaction_id: Uuid,
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub payment_method_id: Option<String>,
}

pub struct ReviewAmlAlert {
    pub alert_id: Uuid,
    pub reviewer_id: Uuid,
    pub decision: AmlAlertDecision,
    pub _notes: Option<String>,
}

pub enum AmlAlertDecision {
    Escalated,
    ClosedFalsePositive,
}

// ─── Results ────────────────────────────────────────────────────────────────

pub struct SubmitKybEvidenceResult {
    pub kyb_case: KybCase,
}

pub struct ReviewKybCaseResult {
    pub kyb_case: KybCase,
}

pub struct ScanTransactionResult {
    pub alerts: Vec<AmlAlert>,
    pub blocked: bool,
}

pub struct ReviewAmlAlertResult {
    pub alert: AmlAlert,
}
