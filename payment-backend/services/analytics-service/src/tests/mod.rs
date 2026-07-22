//! Analytics Service TDD tests
//!
//! Tests cover:
//! 1. Event ingestion (PaymentAuthorized, PaymentFailed)
//! 2. Authorization rates (hourly aggregation)
//! 3. Decline reason breakdown
//! 4. Settlement status (matched/unmatched)
//! 5. Fee analysis
//! 6. Chargeback trends (30/90 day)
//! 7. Scheme compliance (Visa/Mastercard thresholds)
//! 8. Fraud analysis (by BIN, country, amount range)
//! 9. Revenue recovery (via failover)
//! 10. Health check (data staleness)

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use crate::repository::AnalyticsRepository;
use chrono::{Duration, Utc};
use uuid::Uuid;

fn setup() -> AnalyticsPipeline {
    AnalyticsPipeline::new()
}

/// Helper: ingest a PaymentAuthorized event.
async fn ingest_auth(pipeline: &AnalyticsPipeline, acquirer: &str, scheme: &str, amount: i64) {
    pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: "PaymentAuthorized".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some(acquirer.into()),
            card_scheme: Some(scheme.into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(amount),
            decline_reason: None,
            latency_ms: Some(120),
            acquirer_fee: Some((amount as f64 * 0.02) as i64),
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: Some("411111".into()),
            country_code: Some("US".into()),
            merchant_id: Some("merchant_1".into()),
            failover_routed: Some(false),
        })
        .await
        .unwrap();
}

/// Helper: ingest a PaymentFailed event.
async fn ingest_fail(
    pipeline: &AnalyticsPipeline,
    acquirer: &str,
    scheme: &str,
    decline_reason: &str,
) {
    pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: "PaymentFailed".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some(acquirer.into()),
            card_scheme: Some(scheme.into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(5000),
            decline_reason: Some(decline_reason.into()),
            latency_ms: Some(200),
            acquirer_fee: None,
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: Some("550000".into()),
            country_code: Some("AE".into()),
            merchant_id: Some("merchant_1".into()),
            failover_routed: Some(false),
        })
        .await
        .unwrap();
}

/// Helper: ingest a ChargebackReceived event.
async fn ingest_chargeback(
    pipeline: &AnalyticsPipeline,
    scheme: &str,
    amount: i64,
    reason: &str,
) {
    pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: "ChargebackReceived".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some("acquirer_1".into()),
            card_scheme: Some(scheme.into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(amount),
            decline_reason: None,
            latency_ms: None,
            acquirer_fee: None,
            chargeback_amount: Some(amount),
            chargeback_reason: Some(reason.into()),
            fraud_score: Some(0.9),
            bin: Some("411111".into()),
            country_code: Some("US".into()),
            merchant_id: Some("merchant_1".into()),
            failover_routed: Some(false),
        })
        .await
        .unwrap();
}

/// Helper: ingest a settlement event.
async fn ingest_settlement(
    pipeline: &AnalyticsPipeline,
    event_type: &str,
    acquirer: &str,
    amount: i64,
) {
    pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: event_type.into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some(acquirer.into()),
            card_scheme: Some("visa".into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(amount),
            decline_reason: None,
            latency_ms: None,
            acquirer_fee: None,
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: None,
            country_code: None,
            merchant_id: Some("merchant_1".into()),
            failover_routed: Some(false),
        })
        .await
        .unwrap();
}

/// Helper: ingest fee event.
async fn ingest_fee(pipeline: &AnalyticsPipeline, acquirer: &str, fee: i64) {
    pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: "FeeRecorded".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some(acquirer.into()),
            card_scheme: Some("visa".into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(10000),
            decline_reason: None,
            latency_ms: None,
            acquirer_fee: Some(fee),
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: None,
            country_code: None,
            merchant_id: None,
            failover_routed: Some(false),
        })
        .await
        .unwrap();
}

/// Helper: ingest a failover routing decision.
async fn ingest_failover(pipeline: &AnalyticsPipeline, amount: i64) {
    pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: "RoutingDecision".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some("fallback_acquirer".into()),
            card_scheme: Some("visa".into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(amount),
            decline_reason: None,
            latency_ms: None,
            acquirer_fee: None,
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: None,
            country_code: None,
            merchant_id: None,
            failover_routed: Some(true),
        })
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// Test 1: Event ingestion
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ingest_event() {
    let pipeline = setup();
    let event = pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: "PaymentAuthorized".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some("acquirer_1".into()),
            card_scheme: Some("visa".into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(10000),
            decline_reason: None,
            latency_ms: Some(150),
            acquirer_fee: Some(200),
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: Some("411111".into()),
            country_code: Some("US".into()),
            merchant_id: Some("merchant_1".into()),
            failover_routed: Some(false),
        })
        .await
        .unwrap();

    assert_eq!(event.event_type, "PaymentAuthorized");
    assert_eq!(event.acquirer_id, Some("acquirer_1".into()));
    assert_eq!(event.card_scheme, Some("visa".into()));
    assert_eq!(event.amount_minor_units, Some(10000));
    assert!(event.ingested_at <= chrono::Utc::now());

    let count = pipeline.repo.read().await.event_count().await;
    assert_eq!(count, 1);
}

// ---------------------------------------------------------------------------
// Test 2: Authorization rates
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_authorization_rates() {
    let pipeline = setup();

    // Ingest events for two acquirers
    ingest_auth(&pipeline, "acquirer_a", "visa", 10000).await;
    ingest_auth(&pipeline, "acquirer_a", "visa", 5000).await;
    ingest_fail(&pipeline, "acquirer_a", "visa", "insufficient_funds").await;
    ingest_auth(&pipeline, "acquirer_b", "mastercard", 20000).await;

    let now = Utc::now();
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);

    let rates = pipeline
        .api
        .authorization_rates(AuthRateQuery {
            date_range: DateRangeFilter { start, end },
            acquirer_ids: None,
            card_schemes: None,
        })
        .await
        .unwrap();

    // Should have two rows (acquirer_a/visa and acquirer_b/mastercard)
    assert_eq!(rates.len(), 2);

    let a_visa = rates.iter().find(|r| r.acquirer_id == "acquirer_a").unwrap();
    assert_eq!(a_visa.approved_count, 2);
    assert_eq!(a_visa.declined_count, 1);
    assert_eq!(a_visa.total_count, 3);
    assert!((a_visa.auth_rate_pct - 66.67).abs() < 0.01);

    let b_mc = rates
        .iter()
        .find(|r| r.acquirer_id == "acquirer_b")
        .unwrap();
    assert_eq!(b_mc.approved_count, 1);
    assert_eq!(b_mc.declined_count, 0);
    assert_eq!(b_mc.auth_rate_pct, 100.0);
}

// ---------------------------------------------------------------------------
// Test 3: Decline reason breakdown
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_decline_reasons() {
    let pipeline = setup();

    ingest_fail(&pipeline, "acquirer_a", "visa", "insufficient_funds").await;
    ingest_fail(&pipeline, "acquirer_a", "visa", "insufficient_funds").await;
    ingest_fail(&pipeline, "acquirer_a", "mastercard", "do_not_honor").await;

    let now = Utc::now();
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);

    let reasons = pipeline
        .api
        .decline_reasons(DeclineReasonQuery {
            date_range: DateRangeFilter { start, end },
            acquirer_ids: None,
        })
        .await
        .unwrap();

    assert_eq!(reasons.len(), 2);
    let insufficient = reasons
        .iter()
        .find(|r| r.decline_reason == "insufficient_funds")
        .unwrap();
    assert_eq!(insufficient.count, 2);
    assert!((insufficient.percentage_pct - 66.67).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// Test 4: Settlement status
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_settlement_status() {
    let pipeline = setup();

    ingest_settlement(&pipeline, "SettlementMatched", "acquirer_a", 10000).await;
    ingest_settlement(&pipeline, "SettlementMatched", "acquirer_a", 5000).await;
    ingest_settlement(&pipeline, "SettlementUnmatched", "acquirer_b", 2000).await;

    let now = Utc::now();
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);

    let statuses = pipeline.api.settlement_status(start, end).await.unwrap();
    assert_eq!(statuses.len(), 2);

    let matched = statuses.iter().find(|s| s.status == "matched").unwrap();
    assert_eq!(matched.count, 2);
    assert_eq!(matched.total_amount_minor_units, 15000);

    let unmatched = statuses
        .iter()
        .find(|s| s.status == "unmatched")
        .unwrap();
    assert_eq!(unmatched.count, 1);
    assert_eq!(unmatched.total_amount_minor_units, 2000);
}

// ---------------------------------------------------------------------------
// Test 5: Fee analysis
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fee_analysis() {
    let pipeline = setup();

    ingest_fee(&pipeline, "acquirer_a", 200).await;
    ingest_fee(&pipeline, "acquirer_a", 250).await;
    ingest_fee(&pipeline, "acquirer_b", 150).await;

    let now = Utc::now();
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);

    let fees = pipeline
        .api
        .fee_analysis(FeeAnalysisQuery {
            date_range: DateRangeFilter { start, end },
            acquirer_ids: None,
        })
        .await
        .unwrap();

    assert_eq!(fees.len(), 2);
    let a_fee = fees.iter().find(|f| f.acquirer_id == "acquirer_a").unwrap();
    assert_eq!(a_fee.total_fees_minor_units, 450);
    assert_eq!(a_fee.transaction_count, 2);
    assert!((a_fee.avg_fee_per_transaction - 225.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// Test 6: Chargeback trends
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_chargeback_trends() {
    let pipeline = setup();

    // Ingest normal transactions
    ingest_auth(&pipeline, "acquirer_1", "visa", 10000).await;
    ingest_auth(&pipeline, "acquirer_1", "visa", 5000).await;
    ingest_auth(&pipeline, "acquirer_1", "mastercard", 20000).await;

    // Ingest chargebacks
    ingest_chargeback(&pipeline, "visa", 10000, "fraud").await;

    let trends = pipeline
        .api
        .chargeback_trends(ChargebackTrendQuery {
            period_days: 30,
            card_schemes: None,
        })
        .await
        .unwrap();

    // Should have visa and mastercard
    let visa_trend = trends.iter().find(|t| t.card_scheme == "visa").unwrap();
    assert_eq!(visa_trend.chargeback_count, 1);
    assert_eq!(visa_trend.transaction_count, 2);
    assert!((visa_trend.chargeback_rate_pct - 50.0).abs() < 0.01);

    let mc_trend = trends
        .iter()
        .find(|t| t.card_scheme == "mastercard")
        .unwrap();
    assert_eq!(mc_trend.chargeback_count, 0);
    assert_eq!(mc_trend.transaction_count, 1);
}

// ---------------------------------------------------------------------------
// Test 7: Scheme compliance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_scheme_compliance() {
    let pipeline = setup();

    // Ingest transactions under Visa threshold (1.5%)
    for _ in 0..100 {
        ingest_auth(&pipeline, "acquirer_1", "visa", 10000).await;
    }
    // Only 1 chargeback — rate = 1% (under 1.5% threshold)
    ingest_chargeback(&pipeline, "visa", 10000, "fraud").await;

    let compliance = pipeline.api.scheme_compliance().await.unwrap();

    let visa = compliance.iter().find(|c| c.card_scheme == "visa").unwrap();
    assert_eq!(visa.threshold_name, "Chargeback Rate");
    assert_eq!(visa.threshold_pct, 1.5);
    assert!((visa.current_rate_pct - 1.0).abs() < 0.01);
    assert!(!visa.is_breaching);
}

// ---------------------------------------------------------------------------
// Test 8: Fraud analysis
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fraud_analysis_by_country() {
    let pipeline = setup();

    // Ingest events from different countries
    for _ in 0..10 {
        pipeline
            .api
            .ingest_event(IngestAnalyticsEvent {
                event_type: "PaymentAuthorized".into(),
                payment_intent_id: Some(Uuid::now_v7()),
                operator_id: Some(Uuid::now_v7()),
                acquirer_id: Some("acquirer_1".into()),
                card_scheme: Some("visa".into()),
                currency: Some("USD".into()),
                amount_minor_units: Some(10000),
                decline_reason: None,
                latency_ms: Some(100),
                acquirer_fee: None,
                chargeback_amount: None,
                chargeback_reason: None,
                fraud_score: Some(0.1),
                bin: Some("411111".into()),
                country_code: Some("US".into()),
                merchant_id: None,
                failover_routed: Some(false),
            })
            .await
            .unwrap();
    }

    // 5 high-fraud events from NG
    for _ in 0..5 {
        pipeline
            .api
            .ingest_event(IngestAnalyticsEvent {
                event_type: "ChargebackReceived".into(),
                payment_intent_id: Some(Uuid::now_v7()),
                operator_id: Some(Uuid::now_v7()),
                acquirer_id: Some("acquirer_1".into()),
                card_scheme: Some("visa".into()),
                currency: Some("USD".into()),
                amount_minor_units: Some(10000),
                decline_reason: None,
                latency_ms: None,
                acquirer_fee: None,
                chargeback_amount: None,
                chargeback_reason: Some("fraud".into()),
                fraud_score: Some(0.95),
                bin: Some("000000".into()),
                country_code: Some("NG".into()),
                merchant_id: None,
                failover_routed: Some(false),
            })
            .await
            .unwrap();
    }

    let now = Utc::now();
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);

    let fraud = pipeline
        .api
        .fraud_analysis(FraudAnalysisQuery {
            date_range: DateRangeFilter { start, end },
            dimension: FraudDimension::Country,
        })
        .await
        .unwrap();

    let ng = fraud.iter().find(|f| f.dimension_value == "NG").unwrap();
    assert!(ng.fraud_count >= 5);
    assert!(ng.fraud_rate_pct > 50.0);

    let us = fraud.iter().find(|f| f.dimension_value == "US").unwrap();
    assert_eq!(us.fraud_count, 0);
}

// ---------------------------------------------------------------------------
// Test 9: Revenue recovery
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_revenue_recovery() {
    let pipeline = setup();

    // Ingest some failover-routed transactions
    ingest_failover(&pipeline, 50000).await;
    ingest_failover(&pipeline, 75000).await;
    // Non-failover routing decision
    pipeline
        .api
        .ingest_event(IngestAnalyticsEvent {
            event_type: "RoutingDecision".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some("primary_acquirer".into()),
            card_scheme: Some("visa".into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(30000),
            decline_reason: None,
            latency_ms: None,
            acquirer_fee: None,
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: None,
            country_code: None,
            merchant_id: None,
            failover_routed: Some(false),
        })
        .await
        .unwrap();

    let now = Utc::now();
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);

    let recovery = pipeline.api.revenue_recovery(start, end).await.unwrap();
    assert_eq!(recovery.len(), 1);
    assert_eq!(recovery[0].failover_count, 2);
    assert_eq!(recovery[0].recovered_amount_minor_units, 125000);
    // Estimated savings = 2% of 125000 = 2500
    assert_eq!(recovery[0].estimated_savings_minor_units, 2500);
}

// ---------------------------------------------------------------------------
// Test 10: Health check
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_check_ok() {
    let pipeline = setup();
    // No data yet, but health check should pass (empty is not stale)
    let result = pipeline.api.health_check(3600).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_health_check_with_recent_data() {
    let pipeline = setup();
    ingest_auth(&pipeline, "acquirer_1", "visa", 10000).await;

    // 1 second threshold — should still be OK since we just ingested
    let result = pipeline.api.health_check(3600).await;
    assert!(result.is_ok());
}
