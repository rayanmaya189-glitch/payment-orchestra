//! Financial tests: authorization rates, decline reasons, settlement status, fee analysis.

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use chrono::{Duration, Utc};

use super::{ingest_auth, ingest_fail, ingest_fee, ingest_settlement, setup};

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
