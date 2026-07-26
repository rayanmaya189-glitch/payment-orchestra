//! Payment Link TDD tests — BC-07
//!
//! Spec test cases:

mod pg_repository_tests;

// Spec test cases:
// - test_create_payment_link: basic creation with token prefix and status
// - test_expired_payment_link_returns_410: expiry detection
// - test_payment_link_token_has_minimum_entropy: 128-bit token check
// - test_resolve_payment_link: successful checkout flow
// - test_double_resolve_rejected: idempotency / used link check
// - test_cancel_payment_link: manual cancellation
// - test_expire_overdue_links: batch expiry job
// - test_create_link_invalid_amount: validation

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> PaymentLinkPipeline {
    PaymentLinkPipeline::new()
}

fn make_create_cmd() -> CreatePaymentLinkCommand {
    CreatePaymentLinkCommand {
        operator_id: Uuid::now_v7(),
        amount_minor_units: 5000,
        currency: "AED".to_string(),
        description: Some("Order #123".into()),
        invoice_id: None,
        expires_in_days: Some(30),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_payment_link() {
    let pipeline = setup();
    let cmd = make_create_cmd();
    let result = pipeline.api.create_payment_link(cmd).await.unwrap();

    assert!(result.token.starts_with("plink_"), "Token should start with plink_");
    assert_eq!(result.status, PaymentLinkStatus::Active);
    assert_eq!(result.amount_minor_units, 5000);
    assert_eq!(result.currency, "AED");
    assert_eq!(result.description.as_deref(), Some("Order #123"));
    assert!(result.expires_at > chrono::Utc::now());
}

#[tokio::test]
async fn test_payment_link_token_has_minimum_entropy() {
    let pipeline = setup();
    let result = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();

    // Token format: "plink_" (6 chars) + 22 base62 chars = 28+ chars
    assert!(result.token.len() >= 28, "Token length {} should be >= 28", result.token.len());

    // Verify uniqueness across multiple calls
    let result2 = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();
    assert_ne!(result.token, result2.token, "Tokens should be unique");
}

#[tokio::test]
async fn test_expired_payment_link_returns_error() {
    let pipeline = setup();
    let cmd = CreatePaymentLinkCommand {
        expires_in_days: Some(0), // expires immediately
        ..make_create_cmd()
    };
    let link = pipeline.api.create_payment_link(cmd).await.unwrap();

    // Verify the link is already expired
    assert!(link.is_expired(&chrono::Utc::now()), "Link with 0-day expiry should be expired");

    // Trying to resolve an expired link should fail
    let resolve_cmd = ResolvePaymentLinkCommand {
        token: link.token.clone(),
        payment_intent_id: Uuid::now_v7(),
    };
    let resolve_result = pipeline.api.resolve_payment_link(resolve_cmd).await;
    assert!(resolve_result.is_err(), "Resolving expired link should fail");
}

#[tokio::test]
async fn test_resolve_payment_link() {
    let pipeline = setup();
    let link = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();
    let payment_intent_id = Uuid::now_v7();

    let resolved = pipeline
        .api
        .resolve_payment_link(ResolvePaymentLinkCommand {
            token: link.token.clone(),
            payment_intent_id,
        })
        .await
        .unwrap();

    assert_eq!(resolved.status, PaymentLinkStatus::Used);
    assert_eq!(resolved.payment_intent_id, Some(payment_intent_id));
    assert!(resolved.used_at.is_some());
}

#[tokio::test]
async fn test_double_resolve_rejected() {
    let pipeline = setup();
    let link = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();
    let payment_intent_id = Uuid::now_v7();

    // First resolve succeeds
    pipeline
        .api
        .resolve_payment_link(ResolvePaymentLinkCommand {
            token: link.token.clone(),
            payment_intent_id,
        })
        .await
        .unwrap();

    // Second resolve should fail
    let second = pipeline
        .api
        .resolve_payment_link(ResolvePaymentLinkCommand {
            token: link.token.clone(),
            payment_intent_id: Uuid::now_v7(),
        })
        .await;
    assert!(second.is_err(), "Double resolve should be rejected");
}

#[tokio::test]
async fn test_cancel_payment_link() {
    let pipeline = setup();
    let link = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();

    let cancelled = pipeline
        .api
        .cancel_payment_link(CancelPaymentLinkCommand {
            payment_link_id: link.payment_link_id,
            reason: Some("Customer requested".into()),
        })
        .await
        .unwrap();

    assert_eq!(cancelled.status, PaymentLinkStatus::Cancelled);
}

#[tokio::test]
async fn test_cancel_used_link_rejected() {
    let pipeline = setup();
    let link = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();

    // Resolve first
    pipeline
        .api
        .resolve_payment_link(ResolvePaymentLinkCommand {
            token: link.token.clone(),
            payment_intent_id: Uuid::now_v7(),
        })
        .await
        .unwrap();

    // Cancelling a used link should fail
    let cancel_result = pipeline
        .api
        .cancel_payment_link(CancelPaymentLinkCommand {
            payment_link_id: link.payment_link_id,
            reason: None,
        })
        .await;
    assert!(cancel_result.is_err(), "Cancelling used link should fail");
}

#[tokio::test]
async fn test_expire_overdue_links() {
    let pipeline = setup();

    // Create a link that expires immediately
    let cmd = CreatePaymentLinkCommand {
        expires_in_days: Some(0),
        ..make_create_cmd()
    };
    pipeline.api.create_payment_link(cmd).await.unwrap();

    // Run expiry job
    let expired = pipeline.api.expire_overdue_links().await.unwrap();
    assert_eq!(expired.len(), 1, "Should expire exactly 1 link");
    assert_eq!(expired[0].status, PaymentLinkStatus::Expired);
}

#[tokio::test]
async fn test_create_link_invalid_amount_rejected() {
    let pipeline = setup();
    let cmd = CreatePaymentLinkCommand {
        amount_minor_units: 0,
        ..make_create_cmd()
    };
    let result = pipeline.api.create_payment_link(cmd).await;
    assert!(result.is_err(), "Zero amount should be rejected");

    let cmd = CreatePaymentLinkCommand {
        amount_minor_units: -100,
        ..make_create_cmd()
    };
    let result = pipeline.api.create_payment_link(cmd).await;
    assert!(result.is_err(), "Negative amount should be rejected");
}

#[tokio::test]
async fn test_query_payment_link_by_id() {
    let pipeline = setup();
    let link = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();

    let fetched = pipeline.api.get_payment_link(link.payment_link_id).await.unwrap();
    assert_eq!(fetched.payment_link_id, link.payment_link_id);
    assert_eq!(fetched.token, link.token);
}

#[tokio::test]
async fn test_query_payment_link_by_token() {
    let pipeline = setup();
    let link = pipeline.api.create_payment_link(make_create_cmd()).await.unwrap();

    let fetched = pipeline
        .api
        .get_payment_link_by_token(&link.token)
        .await
        .unwrap();
    assert_eq!(fetched.payment_link_id, link.payment_link_id);
}

#[tokio::test]
async fn test_query_nonexistent_link_not_found() {
    let pipeline = setup();
    let result = pipeline.api.get_payment_link(Uuid::now_v7()).await;
    assert!(result.is_err(), "Nonexistent link should return error");

    let result = pipeline.api.get_payment_link_by_token("plink_nonexistent").await;
    assert!(result.is_err(), "Nonexistent token should return error");
}
