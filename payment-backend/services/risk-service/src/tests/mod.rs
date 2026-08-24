//! Fraud & Risk Scoring TDD tests — BC-11
//!
//! Spec test cases:

mod pg_repository_tests;

// Spec test cases:
// - test_low_risk_transaction: small amount, same country → low score
// - test_high_risk_transaction: high amount + geo mismatch → high score
// - test_very_high_amount: very high amount threshold
// - test_geo_mismatch: different billing/shipping countries
// - test_new_payment_method: newly created token
// - test_risk_score_clamped: multiple rules max out at 1.0
// - test_invalid_card_bin_rejected: validation
// - test_unsupported_currency_rejected: validation
// - test_get_assessment: query by payment intent

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> RiskPipeline {
    RiskPipeline::new()
}

fn low_risk_cmd() -> AssessRiskCommand {
    AssessRiskCommand {
        payment_intent_id: Uuid::now_v7(),
        amount_minor_units: 5000, // 50 AED — low amount
        currency: "AED".into(),
        card_bin: "411111".into(), // standard Visa test BIN
        billing_country: "AE".into(),
        shipping_country: Some("AE".into()), // same country
        is_new_payment_method: false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_low_risk_transaction() {
    let pipeline = setup();
    let result = pipeline.api.assess_risk(low_risk_cmd()).await.unwrap();

    assert!(
        result.risk_score < 0.3,
        "Low risk transaction should have score < 0.3, got {}",
        result.risk_score
    );
    assert_eq!(result.risk_level, RiskLevel::Low);
    assert!(result.risk_factors.is_empty());
}

#[tokio::test]
async fn test_high_amount_increases_score() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        amount_minor_units: 150_000, // 1500 AED — above high_amount threshold (100k)
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await.unwrap();

    assert!(
        result.risk_score >= 0.25,
        "High amount should add 0.25 to score, got {}",
        result.risk_score
    );
    assert!(
        result.risk_factors.contains(&"High transaction amount".into())
    );
}

#[tokio::test]
async fn test_very_high_amount_triggers_both_rules() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        amount_minor_units: 600_000, // 6000 AED — triggers both amount rules
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await.unwrap();

    // High amount (0.25) + Very high amount (0.40) = 0.65
    assert!(
        (result.risk_score - 0.65).abs() < 0.01,
        "Very high amount should score ~0.65, got {}",
        result.risk_score
    );
    assert_eq!(result.risk_level, RiskLevel::Medium);
}

#[tokio::test]
async fn test_geo_mismatch_increases_score() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        shipping_country: Some("US".into()), // different from billing (AE)
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await.unwrap();

    assert!(
        result.risk_score >= 0.30,
        "Geo mismatch should add 0.30 to score, got {}",
        result.risk_score
    );
    assert!(
        result.risk_factors.contains(&"Billing and shipping country mismatch".into())
    );
}

#[tokio::test]
async fn test_new_payment_method_increases_score() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        is_new_payment_method: true,
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await.unwrap();

    assert!(
        result.risk_score >= 0.15,
        "New payment method should add 0.15 to score, got {}",
        result.risk_score
    );
}

#[tokio::test]
async fn test_geo_mismatch_and_high_amount_stack() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        amount_minor_units: 150_000, // triggers high_amount (0.25)
        shipping_country: Some("US".into()), // triggers geo_mismatch (0.30)
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await.unwrap();

    // 0.25 + 0.30 = 0.55
    assert!(
        (result.risk_score - 0.55).abs() < 0.01,
        "Combined rules should score ~0.55, got {}",
        result.risk_score
    );
    assert_eq!(result.risk_level, RiskLevel::Medium);
}

#[tokio::test]
async fn test_risk_score_clamped_at_one() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        amount_minor_units: 600_000, // very high (0.40) + high (0.25)
        shipping_country: Some("US".into()), // geo (0.30)
        is_new_payment_method: true, // new method (0.15)
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await.unwrap();

    // 0.40 + 0.25 + 0.30 + 0.15 = 1.10 → clamped to 1.0
    assert!(
        (result.risk_score - 1.0).abs() < 0.01,
        "Score should be clamped at 1.0, got {}",
        result.risk_score
    );
    assert_eq!(result.risk_level, RiskLevel::Critical);
}

#[tokio::test]
async fn test_invalid_card_bin_rejected() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        card_bin: "123".into(), // too short
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await;
    assert!(result.is_err(), "Invalid card BIN should be rejected");

    let cmd = AssessRiskCommand {
        card_bin: "41111A".into(), // non-digit
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await;
    assert!(result.is_err(), "Non-digit BIN should be rejected");
}

#[tokio::test]
async fn test_unsupported_currency_rejected() {
    let pipeline = setup();
    let cmd = AssessRiskCommand {
        currency: "XYZ".into(),
        ..low_risk_cmd()
    };
    let result = pipeline.api.assess_risk(cmd).await;
    assert!(result.is_err(), "Unsupported currency should be rejected");
}

#[tokio::test]
async fn test_get_assessment_by_payment_intent() {
    let pipeline = setup();
    let payment_intent_id = Uuid::now_v7();
    let cmd = AssessRiskCommand {
        payment_intent_id,
        ..low_risk_cmd()
    };
    let assessment = pipeline.api.assess_risk(cmd).await.unwrap();

    let fetched = pipeline
        .api
        .get_assessment(payment_intent_id)
        .await
        .unwrap();

    assert_eq!(fetched.risk_assessment_id, assessment.risk_assessment_id);
    assert_eq!(fetched.risk_score, assessment.risk_score);
}

#[tokio::test]
async fn test_get_nonexistent_assessment() {
    let pipeline = setup();
    let result = pipeline
        .api
        .get_assessment(Uuid::now_v7())
        .await;
    assert!(result.is_err(), "Nonexistent assessment should error");
}

#[tokio::test]
async fn test_get_default_rules() {
    let pipeline = setup();
    let rules = pipeline.api.get_default_rules().await;
    assert_eq!(rules.len(), 4);
    assert!(rules.iter().any(|r| r.rule_id == "high_amount"));
    assert!(rules.iter().any(|r| r.rule_id == "geo_mismatch"));
    assert!(rules.iter().any(|r| r.rule_id == "new_payment_method"));
}
