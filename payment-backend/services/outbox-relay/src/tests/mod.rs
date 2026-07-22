//! Outbox Relay TDD tests

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

fn setup() -> OutboxRelayPipeline { OutboxRelayPipeline::new() }

#[tokio::test]
async fn test_append_entry() {
    let pipeline = setup();
    let entry = pipeline.api.append_entry(AppendEntry {
        aggregate_type: "PaymentIntent".into(),
        aggregate_id: Uuid::now_v7(),
        event_type: "PaymentCreated".into(),
        event_version: 1,
        payload: vec![1, 2, 3],
    }).await.unwrap();

    assert_eq!(entry.event_type, "PaymentCreated");
    assert!(entry.published_at.is_none());
}

#[tokio::test]
async fn test_poll_and_publish() {
    let pipeline = setup();

    pipeline.api.append_entry(AppendEntry {
        aggregate_type: "PaymentIntent".into(),
        aggregate_id: Uuid::now_v7(),
        event_type: "PaymentAuthorized".into(),
        event_version: 1,
        payload: vec![],
    }).await.unwrap();

    pipeline.api.append_entry(AppendEntry {
        aggregate_type: "Refund".into(),
        aggregate_id: Uuid::now_v7(),
        event_type: "RefundCreated".into(),
        event_version: 1,
        payload: vec![],
    }).await.unwrap();

    let result = pipeline.api.poll_and_publish().await.unwrap();
    assert_eq!(result.published_count, 2);
}

#[tokio::test]
async fn test_poll_only_unpublished() {
    let pipeline = setup();

    let entry = pipeline.api.append_entry(AppendEntry {
        aggregate_type: "PaymentIntent".into(),
        aggregate_id: Uuid::now_v7(),
        event_type: "PaymentCreated".into(),
        event_version: 1,
        payload: vec![],
    }).await.unwrap();

    // Mark one as published
    pipeline.api.mark_published(MarkPublished { outbox_id: entry.outbox_id }).await.unwrap();

    pipeline.api.append_entry(AppendEntry {
        aggregate_type: "PaymentIntent".into(),
        aggregate_id: Uuid::now_v7(),
        event_type: "PaymentCaptured".into(),
        event_version: 1,
        payload: vec![],
    }).await.unwrap();

    let unpublished = pipeline.api.list_unpublished().await.unwrap();
    assert_eq!(unpublished.len(), 1);
    assert_eq!(unpublished[0].event_type, "PaymentCaptured");
}

#[tokio::test]
async fn test_relay_start_stop() {
    let pipeline = setup();
    pipeline.api.start_relay().await.unwrap();
    pipeline.api.stop_relay().await.unwrap();
}

#[tokio::test]
async fn test_duplicate_start_rejected() {
    let pipeline = setup();
    pipeline.api.start_relay().await.unwrap();
    let result = pipeline.api.start_relay().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_metrics() {
    let pipeline = setup();

    pipeline.api.append_entry(AppendEntry {
        aggregate_type: "Test".into(),
        aggregate_id: Uuid::now_v7(),
        event_type: "TestEvent".into(),
        event_version: 1,
        payload: vec![],
    }).await.unwrap();

    let metrics = pipeline.api.get_metrics().await.unwrap();
    assert_eq!(metrics.queue_depth, 1);
}

#[tokio::test]
async fn test_get_nonexistent_entry() {
    let pipeline = setup();
    let result = pipeline.api.get_entry(Uuid::now_v7()).await;
    assert!(result.is_err());
}
