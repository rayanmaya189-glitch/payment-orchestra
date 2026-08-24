//! Fee variance tests.

use uuid::Uuid;

use crate::domain::*;
use crate::commands::*;
use crate::events::ReconciliationEvent;
use crate::repository::*;

use super::{setup_handler};

#[tokio::test]
async fn test_fee_variance_within_tolerance() {
    let (handler, _) = setup_handler();
    let pi_id = Uuid::now_v7();
    let link_id = Uuid::now_v7();

    let result = handler.track_fee_variance(TrackFeeVariance {
        payment_intent_id: pi_id,
        acquirer_link_id: link_id,
        estimated_fee_minor: 250,
        actual_fee_minor: 260, // 4% difference, within 5% tolerance
        tolerance_threshold_percent: 5.0,
    }).await.unwrap();

    assert!(result.is_within_tolerance);
}

#[tokio::test]
async fn test_fee_variance_detected() {
    let (handler, _) = setup_handler();
    let pi_id = Uuid::now_v7();
    let link_id = Uuid::now_v7();

    let result = handler.track_fee_variance(TrackFeeVariance {
        payment_intent_id: pi_id,
        acquirer_link_id: link_id,
        estimated_fee_minor: 250,
        actual_fee_minor: 300, // 20% difference, exceeds 5% tolerance
        tolerance_threshold_percent: 5.0,
    }).await.unwrap();

    assert!(!result.is_within_tolerance);
}

#[tokio::test]
async fn test_resolve_fee_variance() {
    let (handler, repo) = setup_handler();
    let pi_id = Uuid::now_v7();
    let link_id = Uuid::now_v7();

    let result = handler.track_fee_variance(TrackFeeVariance {
        payment_intent_id: pi_id,
        acquirer_link_id: link_id,
        estimated_fee_minor: 250,
        actual_fee_minor: 300,
        tolerance_threshold_percent: 5.0,
    }).await.unwrap();

    // Resolve the variance
    let event = handler.resolve_fee_variance(ResolveFeeVarianceCmd {
        variance_id: result.variance_id,
        resolution: "Acquirer confirmed billing correct".into(),
    }).await.unwrap();

    match event {
        ReconciliationEvent::FeeVarianceResolved(e) => {
            assert_eq!(e.variance_id, result.variance_id);
        }
        _ => panic!("Expected FeeVarianceResolved event"),
    }

    // Verify the variance was updated
    let variance = repo.load_fee_variance(result.variance_id).await.unwrap().unwrap();
    assert!(variance.resolved_at.is_some());
}
