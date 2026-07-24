//! Domain model for BC-03 Merchant Compliance.
//! Owns KybCase aggregate, AmlAlert entity, AML rule engine, and SAR reports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── KYB Case Aggregate ─────────────────────────────────────────────────────

/// A Know Your Business case for operator verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KybCase {
    pub kyb_case_id: Uuid,
    pub operator_id: Uuid,
    pub status: KybStatus,
    pub submitted_by: Uuid,
    pub document_ids: Vec<Uuid>,
    pub ocr_extracted_fields: Option<String>,
    pub partner_decision: Option<String>,
    pub rejection_reason: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KybStatus {
    Submitted,
    UnderReview,
    Approved,
    Rejected,
}

impl KybStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::UnderReview => "under_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "submitted" => Some(Self::Submitted),
            "under_review" => Some(Self::UnderReview),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Approved | Self::Rejected)
    }
}

impl KybCase {
    pub fn new(
        operator_id: Uuid,
        submitted_by: Uuid,
        document_ids: Vec<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            kyb_case_id: Uuid::now_v7(),
            operator_id,
            status: KybStatus::Submitted,
            submitted_by,
            document_ids,
            ocr_extracted_fields: None,
            partner_decision: None,
            rejection_reason: None,
            submitted_at: now,
            resolved_at: None,
            updated_at: now,
        }
    }

    pub fn approve(&mut self) -> Result<(), ComplianceError> {
        if self.status.is_terminal() {
            return Err(ComplianceError::KybCaseAlreadyResolved(self.kyb_case_id));
        }
        self.status = KybStatus::Approved;
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn reject(&mut self, reason: String) -> Result<(), ComplianceError> {
        if self.status.is_terminal() {
            return Err(ComplianceError::KybCaseAlreadyResolved(self.kyb_case_id));
        }
        self.status = KybStatus::Rejected;
        self.rejection_reason = Some(reason);
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }
}

// ─── AML Alert Entity ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmlAlert {
    pub alert_id: Uuid,
    pub operator_id: Uuid,
    pub transaction_id: Uuid,
    pub alert_type: AmlAlertType,
    pub severity: AlertSeverity,
    pub rule_id: String,
    pub details: String,
    pub status: AlertStatus,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmlAlertType {
    Structuring,
    Velocity,
    AmountAnomaly,
    RapidSuccession,
}

impl AmlAlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Structuring => "structuring",
            Self::Velocity => "velocity",
            Self::AmountAnomaly => "amount_anomaly",
            Self::RapidSuccession => "rapid_succession",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "structuring" => Some(Self::Structuring),
            "velocity" => Some(Self::Velocity),
            "amount_anomaly" => Some(Self::AmountAnomaly),
            "rapid_succession" => Some(Self::RapidSuccession),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    Open,
    UnderReview,
    Escalated,
    Closed,
}

impl AlertStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::UnderReview => "under_review",
            Self::Escalated => "escalated",
            Self::Closed => "closed",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "under_review" => Some(Self::UnderReview),
            "escalated" => Some(Self::Escalated),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }
}

// ─── AML Rules Engine ───────────────────────────────────────────────────────

pub struct AmlRule {
    pub rule_id: &'static str,
    pub rule_type: AmlRuleType,
    pub severity: AlertSeverity,
}

pub enum AmlRuleType {
    Structuring {
        report_threshold_minor_units: i64,
        near_threshold_percent: f64,
        min_transactions: u32,
        #[allow(dead_code)]
        window_minutes: u32,
    },
    Velocity {
        max_count: u32,
        #[allow(dead_code)]
        window_minutes: u32,
    },
    AmountAnomaly {
        multiplier: f64,
        min_sample_size: u32,
        max_amount_minor_units: i64,
    },
    RapidSuccession {
        max_count: u32,
        #[allow(dead_code)]
        window_seconds: u32,
    },
}

/// Default AML rule set per architecture doc
pub fn default_aml_rules() -> Vec<AmlRule> {
    vec![
        AmlRule {
            rule_id: "AML-R001",
            rule_type: AmlRuleType::Structuring {
                report_threshold_minor_units: 50_000_00, // 50,000 AED
                near_threshold_percent: 0.9,
                min_transactions: 5,
                window_minutes: 60,
            },
            severity: AlertSeverity::High,
        },
        AmlRule {
            rule_id: "AML-R003",
            rule_type: AmlRuleType::Velocity {
                max_count: 20,
                window_minutes: 60,
            },
            severity: AlertSeverity::Medium,
        },
        AmlRule {
            rule_id: "AML-R005",
            rule_type: AmlRuleType::AmountAnomaly {
                multiplier: 10.0,
                min_sample_size: 30,
                max_amount_minor_units: 500_000_00, // 500,000 AED
            },
            severity: AlertSeverity::High,
        },
        AmlRule {
            rule_id: "AML-R007",
            rule_type: AmlRuleType::RapidSuccession {
                max_count: 5,
                window_seconds: 300,
            },
            severity: AlertSeverity::Medium,
        },
    ]
}

// ─── AML Monitor ────────────────────────────────────────────────────────────

pub struct AmlMonitor {
    pub rules: Vec<AmlRule>,
}

impl AmlMonitor {
    pub fn new() -> Self {
        Self {
            rules: default_aml_rules(),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_rules(rules: Vec<AmlRule>) -> Self {
        Self { rules }
    }

    /// Scan a transaction against all AML rules and generate alerts
    pub fn scan(
        &self,
        transaction_id: Uuid,
        operator_id: Uuid,
        amount_minor_units: i64,
        recent_txns: &[RecentTransaction],
        recent_by_method: &[RecentTransaction],
        avg_amount: f64,
    ) -> Vec<AmlAlert> {
        let mut alerts = Vec::new();

        for rule in &self.rules {
            match &rule.rule_type {
                AmlRuleType::Structuring {
                    report_threshold_minor_units,
                    near_threshold_percent,
                    min_transactions,
                    ..
                } => {
                    let threshold = *report_threshold_minor_units;
                    let near_min = (threshold as f64 * near_threshold_percent) as i64;
                    let near_count = recent_txns
                        .iter()
                        .filter(|t| t.amount_minor_units >= near_min && t.amount_minor_units < threshold)
                        .count();
                    if near_count >= *min_transactions as usize {
                        alerts.push(self.create_alert(
                            transaction_id, operator_id, rule,
                            AmlAlertType::Structuring,
                            format!("{} transactions near reporting threshold ({} AED) in 60 min window", near_count, threshold / 100),
                        ));
                    }
                }
                AmlRuleType::Velocity {
                    max_count,
                    ..
                } => {
                    if recent_txns.len() >= *max_count as usize {
                        alerts.push(self.create_alert(
                            transaction_id, operator_id, rule,
                            AmlAlertType::Velocity,
                            format!("{} transactions in window (max: {})", recent_txns.len(), max_count),
                        ));
                    }
                }
                AmlRuleType::AmountAnomaly {
                    multiplier,
                    min_sample_size,
                    max_amount_minor_units,
                } => {
                    if amount_minor_units >= *max_amount_minor_units {
                        alerts.push(self.create_alert(
                            transaction_id, operator_id, rule,
                            AmlAlertType::AmountAnomaly,
                            format!("Transaction amount {} AED exceeds absolute threshold {} AED", amount_minor_units / 100, max_amount_minor_units / 100),
                        ));
                    } else if recent_txns.len() >= *min_sample_size as usize && avg_amount > 0.0
                        && amount_minor_units as f64 > avg_amount * multiplier {
                            alerts.push(self.create_alert(
                                transaction_id, operator_id, rule,
                                AmlAlertType::AmountAnomaly,
                                format!("Transaction amount {} AED is {:.1}x above operator average of {:.2} AED",
                                    amount_minor_units / 100,
                                    amount_minor_units as f64 / avg_amount,
                                    avg_amount / 100.0),
                            ));
                        }
                }
                AmlRuleType::RapidSuccession {
                    max_count,
                    ..
                } => {
                    if recent_by_method.len() >= *max_count as usize {
                        alerts.push(self.create_alert(
                            transaction_id, operator_id, rule,
                            AmlAlertType::RapidSuccession,
                            format!("{} authorizations on same card in {} seconds", recent_by_method.len(), 300),
                        ));
                    }
                }
            }
        }

        alerts
    }

    fn create_alert(
        &self,
        transaction_id: Uuid,
        operator_id: Uuid,
        rule: &AmlRule,
        alert_type: AmlAlertType,
        details: String,
    ) -> AmlAlert {
        AmlAlert {
            alert_id: Uuid::now_v7(),
            operator_id,
            transaction_id,
            alert_type,
            severity: rule.severity.clone(),
            rule_id: rule.rule_id.to_string(),
            details,
            status: AlertStatus::Open,
            reviewed_by: None,
            reviewed_at: None,
            created_at: Utc::now(),
        }
    }
}

impl Default for AmlMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Transaction Data for AML Scanning ──────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecentTransaction {
    pub transaction_id: Uuid,
    pub amount_minor_units: i64,
    pub payment_method_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ─── SAR Report ─────────────────────────────────────────────────────────────

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum SarStatus {
    Draft,
    Submitted,
    Filed,
}

// ─── Error Types ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ComplianceError {
    #[error("KYB case not found: {0}")]
    KybCaseNotFound(Uuid),

    #[error("KYB case already resolved: {0}")]
    KybCaseAlreadyResolved(Uuid),

    #[error("At least one document required")]
    KybNoDocuments,

    #[error("AML alert not found: {0}")]
    AmlAlertNotFound(Uuid),

    #[error("AML alert already reviewed: {0}")]
    AmlAlertAlreadyReviewed(Uuid),

    #[error("SAR generation failed: {0}")]
    SarGenerationFailed(String),

    #[error("Partner API unavailable: {0}")]
    PartnerApiUnavailable(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<ComplianceError> for platform_error::PlatformError {
    fn from(e: ComplianceError) -> Self {
        match e {
            ComplianceError::KybCaseNotFound(id) | ComplianceError::AmlAlertNotFound(id) => {
                platform_error::PlatformError::NotFound { resource: "compliance", id }
            }
            ComplianceError::KybCaseAlreadyResolved(_) | ComplianceError::AmlAlertAlreadyReviewed(_) => {
                platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::IdempotencyKeyConflict,
                )
            }
            ComplianceError::KybNoDocuments | ComplianceError::InvalidRequest(_) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "request".into(),
                        reason: e.to_string(),
                    },
                )
            }
            ComplianceError::SarGenerationFailed(ref msg) => {
                platform_error::PlatformError::Internal(msg.clone())
            }
            ComplianceError::PartnerApiUnavailable(ref msg) => {
                platform_error::PlatformError::Unavailable(msg.clone())
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyb_case_creation() {
        let kase = KybCase::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            vec![Uuid::now_v7()],
        );
        assert_eq!(kase.status, KybStatus::Submitted);
        assert!(!kase.status.is_terminal());
    }

    #[test]
    fn test_kyb_approve() {
        let mut kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![]);
        assert!(kase.approve().is_ok());
        assert_eq!(kase.status, KybStatus::Approved);
        assert!(kase.status.is_terminal());
    }

    #[test]
    fn test_kyb_reject() {
        let mut kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![]);
        assert!(kase.reject("Invalid documents".into()).is_ok());
        assert_eq!(kase.status, KybStatus::Rejected);
        assert_eq!(kase.rejection_reason.unwrap(), "Invalid documents");
    }

    #[test]
    fn test_kyb_double_approve_fails() {
        let mut kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![]);
        kase.approve().unwrap();
        assert!(kase.approve().is_err());
    }

    #[test]
    fn test_aml_monitor_velocity() {
        let monitor = AmlMonitor::new();
        let operator_id = Uuid::now_v7();
        let recent: Vec<RecentTransaction> = (0..25)
            .map(|i| RecentTransaction {
                transaction_id: Uuid::now_v7(),
                amount_minor_units: 1000 + i * 100,
                payment_method_id: None,
                timestamp: Utc::now(),
            })
            .collect();

        let alerts = monitor.scan(
            Uuid::now_v7(),
            operator_id,
            5000,
            &recent,
            &[],
            0.0,
        );

        let velocity_alerts: Vec<_> = alerts.iter().filter(|a| a.alert_type == AmlAlertType::Velocity).collect();
        assert!(!velocity_alerts.is_empty());
    }

    #[test]
    fn test_aml_monitor_no_alert_for_normal_traffic() {
        let monitor = AmlMonitor::new();
        let recent: Vec<RecentTransaction> = (0..3)
            .map(|i| RecentTransaction {
                transaction_id: Uuid::now_v7(),
                amount_minor_units: 1000 + i * 100,
                payment_method_id: None,
                timestamp: Utc::now(),
            })
            .collect();

        let alerts = monitor.scan(
            Uuid::now_v7(),
            Uuid::now_v7(),
            5000,
            &recent,
            &[],
            0.0,
        );

        assert!(alerts.is_empty());
    }

    #[test]
    fn test_kyb_status_transitions() {
        assert!(!KybStatus::Submitted.is_terminal());
        assert!(!KybStatus::UnderReview.is_terminal());
        assert!(KybStatus::Approved.is_terminal());
        assert!(KybStatus::Rejected.is_terminal());
    }
}
