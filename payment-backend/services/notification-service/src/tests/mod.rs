//! Notification Service TDD tests — BC-14
//!
//! Spec tests:
//! - test_send_notification_email: basic send via email
//! - test_send_notification_invalid_template: missing template rejected
//! - test_mark_delivered: successful delivery
//! - test_mark_failed_and_retry: failure → retry flow
//! - test_mark_failed_exhausted: max retries → dead letter
//! - test_retry_notification: reset failed to queued
//! - test_template_rendering: variable substitution
//! - test_find_pending: query
//! - test_find_dead_letter: query
//! - test_list_templates: 5 default templates

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> NotificationPipeline {
    NotificationPipeline::new()
}

fn email_cmd() -> SendNotificationCommand {
    SendNotificationCommand {
        operator_id: Uuid::now_v7(),
        channel: NotificationChannel::Email,
        recipient: "merchant@example.com".into(),
        template_id: "payment_failed".into(),
        payload_json: r#"{"payment_intent_id": "pi_123", "amount": "5000 AED"}"#.into(),
        subject: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_send_notification_email() {
    let pipeline = setup();
    let notification = pipeline.api.send_notification(email_cmd()).await.unwrap();

    assert_eq!(notification.channel, NotificationChannel::Email);
    assert_eq!(notification.recipient, "merchant@example.com");
    assert_eq!(notification.template_id, "payment_failed");
    assert_eq!(notification.status, DeliveryStatus::Queued);
    assert_eq!(notification.retry_count, 0);
}

#[tokio::test]
async fn test_send_notification_invalid_template_rejected() {
    let pipeline = setup();
    let cmd = SendNotificationCommand {
        template_id: "nonexistent_template".into(),
        ..email_cmd()
    };
    let result = pipeline.api.send_notification(cmd).await;
    assert!(result.is_err(), "Invalid template should be rejected");
}

#[tokio::test]
async fn test_mark_delivered() {
    let pipeline = setup();
    let notification = pipeline.api.send_notification(email_cmd()).await.unwrap();

    let delivered = pipeline
        .api
        .mark_delivered(MarkDeliveredCommand {
            notification_id: notification.notification_id,
        })
        .await
        .unwrap();

    assert_eq!(delivered.status, DeliveryStatus::Sent);
    assert!(delivered.sent_at.is_some());
}

#[tokio::test]
async fn test_mark_already_delivered_rejected() {
    let pipeline = setup();
    let notification = pipeline.api.send_notification(email_cmd()).await.unwrap();

    // Mark delivered once
    pipeline
        .api
        .mark_delivered(MarkDeliveredCommand {
            notification_id: notification.notification_id,
        })
        .await
        .unwrap();

    // Mark again should fail
    let result = pipeline
        .api
        .mark_delivered(MarkDeliveredCommand {
            notification_id: notification.notification_id,
        })
        .await;
    assert!(result.is_err(), "Double delivery should fail");
}

#[tokio::test]
async fn test_mark_failed_and_retry() {
    let pipeline = setup();
    let notification = pipeline.api.send_notification(email_cmd()).await.unwrap();

    // Mark as failed
    let failed = pipeline
        .api
        .mark_failed(MarkFailedCommand {
            notification_id: notification.notification_id,
            error: "SMTP connection timeout".into(),
        })
        .await
        .unwrap();

    assert_eq!(failed.status, DeliveryStatus::Failed);
    assert_eq!(failed.retry_count, 1);
    assert_eq!(failed.last_error.as_deref(), Some("SMTP connection timeout"));

    // Retry should reset to queued
    let retried = pipeline
        .api
        .retry_notification(RetryNotificationCommand {
            notification_id: notification.notification_id,
        })
        .await
        .unwrap();

    assert_eq!(retried.status, DeliveryStatus::Queued);
}

#[tokio::test]
async fn test_mark_failed_exhausted() {
    let pipeline = setup();
    let notification = pipeline.api.send_notification(email_cmd()).await.unwrap();

    // Fail 3 times (max_retries = 3)
    for i in 1..=3 {
        let result = pipeline
            .api
            .mark_failed(MarkFailedCommand {
                notification_id: notification.notification_id,
                error: format!("Attempt {} failed", i),
            })
            .await;
        if i < 3 {
            assert_eq!(result.unwrap().status, DeliveryStatus::Failed);
        } else {
            assert_eq!(result.unwrap().status, DeliveryStatus::DeadLetter);
        }
    }

    // Verify dead letter
    let dead_letters = pipeline.api.find_dead_letter().await.unwrap();
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(dead_letters[0].notification_id, notification.notification_id);
}

#[tokio::test]
async fn test_template_rendering() {
    let templates = default_templates();
    let payment_failed = templates
        .iter()
        .find(|t| t.template_id == "payment_failed")
        .unwrap();

    let mut vars = HashMap::new();
    vars.insert("payment_intent_id".into(), "pi_456".into());
    vars.insert("amount".into(), "9999 AED".into());
    vars.insert("operator_name".into(), "Test Merchant".into());

    let rendered = payment_failed.render(&vars);
    assert!(rendered.subject.unwrap().contains("pi_456"));
    assert!(rendered.body.contains("pi_456"));
    assert!(rendered.body.contains("9999 AED"));
}

#[tokio::test]
async fn test_find_pending() {
    let pipeline = setup();

    // Create a queued notification
    let _queued = pipeline.api.send_notification(email_cmd()).await.unwrap();

    // Create another and mark as sent
    let n2 = pipeline.api.send_notification(email_cmd()).await.unwrap();
    pipeline
        .api
        .mark_delivered(MarkDeliveredCommand {
            notification_id: n2.notification_id,
        })
        .await
        .unwrap();

    let pending = pipeline.api.find_pending().await.unwrap();
    assert_eq!(pending.len(), 1, "Should find exactly 1 pending notification");
    assert_eq!(pending[0].status, DeliveryStatus::Queued);
}

#[tokio::test]
async fn test_list_templates() {
    let pipeline = setup();
    let templates = pipeline.api.list_templates().await;
    assert_eq!(templates.len(), 5);
    assert!(templates.iter().any(|t| t.template_id == "payment_failed"));
    assert!(templates.iter().any(|t| t.template_id == "chargeback_received"));
    assert!(templates.iter().any(|t| t.template_id == "subscription_failed"));
    assert!(templates.iter().any(|t| t.template_id == "api_key_expiring"));
    assert!(templates.iter().any(|t| t.template_id == "settlement_unmatched"));
}
