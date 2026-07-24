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

mod financial_tests;
mod fraud_tests;
mod other_tests;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use crate::repository::AnalyticsRepository;
use chrono::{Duration, Utc};
use uuid::Uuid;

pub(crate) fn setup() -> AnalyticsPipeline {
    AnalyticsPipeline::new()
}

/// Helper: ingest a PaymentAuthorized event.
pub(crate) async fn ingest_auth(pipeline: &AnalyticsPipeline, acquirer: &str, scheme: &str, amount: i64) {
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
pub(crate) async fn ingest_fail(
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
pub(crate) async fn ingest_chargeback(
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
pub(crate) async fn ingest_settlement(
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
pub(crate) async fn ingest_fee(pipeline: &AnalyticsPipeline, acquirer: &str, fee: i64) {
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
pub(crate) async fn ingest_failover(pipeline: &AnalyticsPipeline, amount: i64) {
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
