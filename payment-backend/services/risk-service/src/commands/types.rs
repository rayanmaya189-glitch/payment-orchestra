//! Command types for BC-11 Fraud & Risk Scoring

use uuid::Uuid;

use crate::domain::{RiskAssessment, RiskRule};

/// Assess risk for a payment intent synchronously.
pub struct AssessRiskCommand {
    pub payment_intent_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub card_bin: String,
    pub billing_country: String,
    pub shipping_country: Option<String>,
    pub is_new_payment_method: bool,
}

/// Update an active risk rule.
pub struct UpdateRiskRuleCommand {
    pub rule_id: String,
    pub score_increment: Option<f64>,
    pub is_active: Option<bool>,
    pub description: Option<String>,
}
