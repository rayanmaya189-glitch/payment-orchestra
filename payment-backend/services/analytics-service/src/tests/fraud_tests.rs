//! Fraud tests: fraud analysis, scheme compliance, chargeback trends.

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::{ingest_auth, ingest_chargeback, setup};

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
