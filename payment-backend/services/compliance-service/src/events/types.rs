//! Domain event definitions for BC-03 Merchant Compliance.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum ComplianceEvent {
    KybCaseSubmitted(KybCaseSubmitted),
    KybCaseApproved(KybCaseApproved),
    KybCaseRejected(KybCaseRejected),
    AmlAlertCreated(AmlAlertCreated),
}

#[derive(Debug, Clone, Serialize)]
pub struct KybCaseSubmitted {
    pub kyb_case_id: Uuid,
    pub operator_id: Uuid,
    pub document_count: u32,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KybCaseApproved {
    pub kyb_case_id: Uuid,
    pub operator_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KybCaseRejected {
    pub kyb_case_id: Uuid,
    pub operator_id: Uuid,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AmlAlertCreated {
    pub alert_id: Uuid,
    pub operator_id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub occurred_at: DateTime<Utc>,
}

impl ComplianceEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::KybCaseSubmitted(_) => "kyb_case_submitted",
            Self::KybCaseApproved(_) => "kyb_case_approved",
            Self::KybCaseRejected(_) => "kyb_case_rejected",
            Self::AmlAlertCreated(_) => "aml_alert_created",
        }
    }
}
