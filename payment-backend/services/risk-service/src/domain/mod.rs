//! Fraud & Risk Scoring domain model — BC-11
//!
//! Rule-based risk scoring for payment transactions.
//! Future: ML-based scoring integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// RiskLevel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.9 {
            Self::Critical
        } else if score >= 0.7 {
            Self::High
        } else if score >= 0.3 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ---------------------------------------------------------------------------
// RiskCondition — conditions that trigger risk score increments
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskCondition {
    /// High transaction amount (minor units threshold).
    HighAmount { threshold_minor: i64 },
    /// Too many transactions in a time window.
    HighVelocity { max_count: u32, window_minutes: u32 },
    /// Card BIN is in a suspicious list.
    SuspiciousBin { bin_prefixes: Vec<String> },
    /// Billing and shipping country mismatch.
    GeoMismatch { apply_billing_shipping_check: bool },
    /// Payment method is newly created (token age).
    NewPaymentMethod { max_age_days: u32 },
}

// ---------------------------------------------------------------------------
// RiskRule
// ---------------------------------------------------------------------------

/// A single risk rule with a condition and score increment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRule {
    pub rule_id: String,
    pub condition: RiskCondition,
    pub score_increment: f64,
    pub description: String,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// RiskAssessment aggregate
// ---------------------------------------------------------------------------

/// Result of a risk assessment for a payment intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub risk_score: f64,
    pub risk_level: RiskLevel,
    pub risk_factors: Vec<String>,
    pub rule_version: String,
    pub assessed_at: DateTime<Utc>,
}

impl RiskAssessment {
    pub fn new(payment_intent_id: Uuid) -> Self {
        Self {
            risk_assessment_id: Uuid::now_v7(),
            payment_intent_id,
            risk_score: 0.0,
            risk_level: RiskLevel::Low,
            risk_factors: Vec::new(),
            rule_version: "1.0".into(),
            assessed_at: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// AssessRiskInput — input to the scoring engine
// ---------------------------------------------------------------------------

/// Input data for assessing transaction risk.
#[derive(Debug, Clone)]
pub struct AssessRiskInput {
    pub payment_intent_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub card_bin: String,
    pub billing_country: String,
    pub shipping_country: Option<String>,
    pub is_new_payment_method: bool,
}

// ---------------------------------------------------------------------------
// RiskStats (for analytics / routing optimization)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskStats {
    pub avg_risk_score: f64,
    pub high_risk_count: u32,
    pub critical_risk_count: u32,
    pub risk_by_bin: HashMap<String, f64>,
    pub risk_by_country: HashMap<String, f64>,
}

// ---------------------------------------------------------------------------
// RiskEngine — the scoring engine
// ---------------------------------------------------------------------------

/// Core risk scoring engine. Evaluates rules against transaction data.
pub struct RiskEngine {
    rules: Vec<RiskRule>,
}

impl Default for RiskEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskEngine {
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
        }
    }

    pub fn with_rules(rules: Vec<RiskRule>) -> Self {
        Self { rules }
    }

    /// Default set of risk rules for initial deployment.
    pub fn default_rules() -> Vec<RiskRule> {
        vec![
            RiskRule {
                rule_id: "high_amount".into(),
                condition: RiskCondition::HighAmount { threshold_minor: 100_000 }, // 1000 AED
                score_increment: 0.25,
                description: "High transaction amount".into(),
                is_active: true,
            },
            RiskRule {
                rule_id: "very_high_amount".into(),
                condition: RiskCondition::HighAmount { threshold_minor: 500_000 }, // 5000 AED
                score_increment: 0.40,
                description: "Very high transaction amount".into(),
                is_active: true,
            },
            RiskRule {
                rule_id: "geo_mismatch".into(),
                condition: RiskCondition::GeoMismatch { apply_billing_shipping_check: true },
                score_increment: 0.30,
                description: "Billing and shipping country mismatch".into(),
                is_active: true,
            },
            RiskRule {
                rule_id: "new_payment_method".into(),
                condition: RiskCondition::NewPaymentMethod { max_age_days: 7 },
                score_increment: 0.15,
                description: "Recently created payment method".into(),
                is_active: true,
            },
        ]
    }

    /// Assess risk for a transaction.
    /// Returns the RiskAssessment with score and factors.
    pub fn assess(&self, input: &AssessRiskInput) -> RiskAssessment {
        let mut assessment = RiskAssessment::new(input.payment_intent_id);
        let mut total_score = 0.0;

        for rule in &self.rules {
            if !rule.is_active {
                continue;
            }

            if self.evaluate_condition(&rule.condition, input) {
                total_score += rule.score_increment;
                assessment.risk_factors.push(rule.description.clone());
            }
        }

        // Clamp score between 0.0 and 1.0
        let clamped = total_score.clamp(0.0, 1.0);
        assessment.risk_score = clamped;
        assessment.risk_level = RiskLevel::from_score(clamped);

        assessment
    }

    /// Evaluate a single risk condition against input data.
    fn evaluate_condition(&self, condition: &RiskCondition, input: &AssessRiskInput) -> bool {
        match condition {
            RiskCondition::HighAmount { threshold_minor } => {
                input.amount_minor_units >= *threshold_minor
            }
            RiskCondition::HighVelocity { .. } => {
                // NOTE: HighVelocity requires checking transaction history.
                // In Phase 1, this requires external data (Redis counter).
                false
            }
            RiskCondition::SuspiciousBin { bin_prefixes } => {
                bin_prefixes.iter().any(|prefix| input.card_bin.starts_with(prefix))
            }
            RiskCondition::GeoMismatch { .. } => {
                if let Some(ref shipping) = input.shipping_country {
                    input.billing_country != *shipping
                } else {
                    false
                }
            }
            RiskCondition::NewPaymentMethod { .. } => {
                input.is_new_payment_method
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum RiskError {
    #[error("Risk assessment not found for payment intent: {0}")]
    NotFound(Uuid),
    #[error("Invalid card BIN: must be 6 digits, got {0}")]
    InvalidCardBin(String),
    #[error("Unsupported currency: {0}")]
    UnsupportedCurrency(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}
