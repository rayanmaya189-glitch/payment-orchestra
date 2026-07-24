use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Suspicious Activity Report — generated when AML alerts warrant regulatory filing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SarReport {
    pub report_id: Uuid,
    pub operator_id: Uuid,
    pub alert_ids: Vec<Uuid>,
    pub transactions: Vec<SarTransaction>,
    pub narrative: String,
    pub generated_at: DateTime<Utc>,
    pub status: SarStatus,
}

/// A transaction that is part of a SAR report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SarTransaction {
    pub transaction_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
    pub counterparty: Option<String>,
    pub description: String,
}

/// Status of a SAR report in the filing workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum SarStatus {
    Draft,
    Submitted,
    Filed,
}
