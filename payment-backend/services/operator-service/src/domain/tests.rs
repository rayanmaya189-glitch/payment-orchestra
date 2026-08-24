//! Tests for Operator domain model.

use super::*;

#[test]
fn test_operator_creation() {
    let op = Operator::new(
        Uuid::now_v7(),
        "Acme Corp".into(),
        "CN-12345".into(),
        "AE".into(),
        "admin@acme.com".into(),
        "acme".into(),
    );
    assert_eq!(op.status, OperatorStatus::Pending);
    assert!(!op.status.can_process_live_transactions());
    assert!(!op.status.can_access_sandbox());
}

#[test]
fn test_email_verification() {
    let mut op = Operator::new(
        Uuid::now_v7(),
        "Acme Corp".into(),
        "CN-12345".into(),
        "AE".into(),
        "admin@acme.com".into(),
        "acme".into(),
    );
    assert!(op.verify_email().is_ok());
    assert_eq!(op.status, OperatorStatus::ActiveUnverified);
    assert!(op.status.can_access_sandbox());
}

#[test]
fn test_double_verification_fails() {
    let mut op = Operator::new(
        Uuid::now_v7(),
        "Acme Corp".into(),
        "CN-12345".into(),
        "AE".into(),
        "admin@acme.com".into(),
        "acme".into(),
    );
    op.verify_email().unwrap();
    assert!(op.verify_email().is_err());
}

#[test]
fn test_status_transitions() {
    assert!(OperatorStatus::Pending.can_transition_to(&OperatorStatus::ActiveUnverified));
    assert!(OperatorStatus::ActiveUnverified.can_transition_to(&OperatorStatus::ActiveVerified));
    assert!(OperatorStatus::ActiveVerified.can_transition_to(&OperatorStatus::Suspended));
    assert!(OperatorStatus::Suspended.can_transition_to(&OperatorStatus::ActiveVerified));
    assert!(!OperatorStatus::Pending.can_transition_to(&OperatorStatus::ActiveVerified));
    assert!(!OperatorStatus::ActiveVerified.can_transition_to(&OperatorStatus::Pending));
}

#[test]
fn test_live_transactions_only_verified() {
    assert!(OperatorStatus::ActiveVerified.can_process_live_transactions());
    assert!(!OperatorStatus::Pending.can_process_live_transactions());
    assert!(!OperatorStatus::ActiveUnverified.can_process_live_transactions());
    assert!(!OperatorStatus::Suspended.can_process_live_transactions());
}

#[test]
fn test_status_as_str_roundtrip() {
    for status in &[
        OperatorStatus::Pending,
        OperatorStatus::ActiveUnverified,
        OperatorStatus::ActiveVerified,
        OperatorStatus::Suspended,
        OperatorStatus::ExpiredUnverified,
    ] {
        let s = status.as_str();
        let parsed = OperatorStatus::parse_str(s);
        assert_eq!(parsed, Some(status.clone()));
    }
}
