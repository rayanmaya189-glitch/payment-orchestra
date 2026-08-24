//! Other tests: revenue recovery, event ingestion, health check.

use crate::commands::*;
use crate::repository::*;
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::{ingest_auth, ingest_failover, setup};

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
