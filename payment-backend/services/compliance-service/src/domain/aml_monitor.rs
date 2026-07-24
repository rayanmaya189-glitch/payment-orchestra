use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::aml_alert::{AlertSeverity, AlertStatus, AmlAlert, AmlAlertType};

/// An AML rule for scanning transactions.
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
                report_threshold_minor_units: 5_000_000, // 50,000 AED (in minor units)
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
                max_amount_minor_units: 50_000_000, // 500,000 AED (in minor units)
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

/// Transaction data used for AML scanning input.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecentTransaction {
    pub transaction_id: Uuid,
    pub amount_minor_units: i64,
    pub payment_method_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// AML monitor — scans transactions against configured rules.
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
