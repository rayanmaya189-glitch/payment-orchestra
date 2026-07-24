//! Domain module — merchant-connector-onboarding concepts.
//!
//! File structure (one concept per file per CONVENTIONS.md):
//!
//! - [`error`]               — [`OnboardingError`]
//! - [`status`]              — [`OnboardingStatus`] state machine
//! - [`types`]               — [`ConnectorConfiguration`], [`ApiCredentials`],
//!                             [`WebhookConfiguration`], [`BusinessDetails`]
//! - [`onboarding_request`]  — [`OnboardingRequest`] + validation + defaults

pub mod error;
pub mod onboarding_request;
pub mod status;
pub mod types;

pub use error::*;
pub use onboarding_request::*;
pub use status::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_onboarding_status_valid_transitions() {
        assert!(OnboardingStatus::Draft.can_transition_to(&OnboardingStatus::CredentialsSubmitted));
        assert!(OnboardingStatus::CredentialsSubmitted.can_transition_to(&OnboardingStatus::Testing));
        assert!(OnboardingStatus::Testing.can_transition_to(&OnboardingStatus::Active));
        assert!(OnboardingStatus::Testing.can_transition_to(&OnboardingStatus::CredentialsSubmitted));
        assert!(OnboardingStatus::Active.can_transition_to(&OnboardingStatus::Deactivated));
        assert!(OnboardingStatus::Active.can_transition_to(&OnboardingStatus::Revoked));
        assert!(OnboardingStatus::Deactivated.can_transition_to(&OnboardingStatus::Active));
        assert!(OnboardingStatus::Deactivated.can_transition_to(&OnboardingStatus::Revoked));
    }

    #[test]
    fn test_onboarding_status_invalid_transitions() {
        assert!(!OnboardingStatus::Draft.can_transition_to(&OnboardingStatus::Active));
        assert!(!OnboardingStatus::Draft.can_transition_to(&OnboardingStatus::Revoked));
        assert!(!OnboardingStatus::CredentialsSubmitted.can_transition_to(&OnboardingStatus::Draft));
        assert!(!OnboardingStatus::Active.can_transition_to(&OnboardingStatus::Draft));
        assert!(!OnboardingStatus::Active.can_transition_to(&OnboardingStatus::Testing));
        assert!(!OnboardingStatus::Active.can_transition_to(&OnboardingStatus::CredentialsSubmitted));
    }

    #[test]
    fn test_onboarding_status_is_terminal() {
        assert!(!OnboardingStatus::Draft.is_terminal());
        assert!(!OnboardingStatus::CredentialsSubmitted.is_terminal());
        assert!(!OnboardingStatus::Testing.is_terminal());
        assert!(!OnboardingStatus::Active.is_terminal());
        assert!(!OnboardingStatus::Deactivated.is_terminal());
        assert!(OnboardingStatus::Revoked.is_terminal());
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Unknown), "unknown");
        assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
        assert_eq!(format!("{}", HealthStatus::Degraded), "degraded");
        assert_eq!(format!("{}", HealthStatus::Down), "down");
    }

    #[test]
    fn test_onboarding_request_new() {
        let operator_id = Uuid::now_v7();
        let request = OnboardingRequest::new(
            operator_id,
            "network_international".into(),
            "Test Connector".into(),
            "sandbox".into(),
        );

        assert_eq!(request.operator_id, operator_id);
        assert_eq!(request.connector_id, "network_international");
        assert_eq!(request.display_name, "Test Connector");
        assert_eq!(request.environment, "sandbox");
        assert_eq!(request.status, OnboardingStatus::Draft);
        assert_eq!(request.health_status, HealthStatus::Unknown);
        assert!(request.credentials.is_empty());
        assert!(request.encrypted_credentials.is_empty());
        assert!(request.last_tested_at.is_none());
        assert!(request.last_test_result.is_none());
    }

    #[test]
    fn test_submit_credentials_success() {
        let operator_id = Uuid::now_v7();
        let mut request = OnboardingRequest::new(
            operator_id,
            "network_international".into(),
            "Test".into(),
            "sandbox".into(),
        );

        let connectors = default_connectors();
        let schema = connectors.iter().find(|c| c.connector_id == "network_international").unwrap();

        let mut credentials = HashMap::new();
        credentials.insert("merchant_id".into(), "MER-12345".into());
        credentials.insert("api_key".into(), "abc123def456abc123def456abc12345".into());
        credentials.insert("environment".into(), "sandbox".into());

        assert!(request.submit_credentials(credentials, schema).is_ok());
        assert_eq!(request.status, OnboardingStatus::CredentialsSubmitted);
    }

    #[test]
    fn test_submit_credentials_missing_field() {
        let operator_id = Uuid::now_v7();
        let mut request = OnboardingRequest::new(
            operator_id,
            "network_international".into(),
            "Test".into(),
            "sandbox".into(),
        );

        let connectors = default_connectors();
        let schema = connectors.iter().find(|c| c.connector_id == "network_international").unwrap();

        let credentials = HashMap::new(); // Empty — missing required fields
        assert!(request.submit_credentials(credentials, schema).is_err());
        assert_eq!(request.status, OnboardingStatus::Draft); // unchanged
    }

    #[test]
    fn test_onboarding_lifecycle() {
        let operator_id = Uuid::now_v7();
        let mut request = OnboardingRequest::new(
            operator_id,
            "checkout_com".into(),
            "My Checkout.com".into(),
            "production".into(),
        );

        // Submit credentials
        let connectors = default_connectors();
        let schema = connectors.iter().find(|c| c.connector_id == "checkout_com").unwrap();
        let mut credentials = HashMap::new();
        credentials.insert("secret_key".into(), "sk_live_abcdef123456".into());
        credentials.insert("public_key".into(), "pk_live_abcdef123456".into());
        credentials.insert("environment".into(), "production".into());
        assert!(request.submit_credentials(credentials, schema).is_ok());
        assert_eq!(request.status, OnboardingStatus::CredentialsSubmitted);

        // Start testing
        assert!(request.start_test().is_ok());
        assert_eq!(request.status, OnboardingStatus::Testing);

        // Record test success
        let result = ConnectionTestResult {
            success: true,
            latency_ms: 150,
            error: None,
            merchant_name: Some("Test Merchant".into()),
            permissions: vec!["authorize".into(), "capture".into()],
        };
        assert!(request.record_test_success(result).is_ok());
        assert_eq!(request.status, OnboardingStatus::Active);
        assert_eq!(request.health_status, HealthStatus::Healthy);
        assert!(request.last_tested_at.is_some());
        assert!(request.last_test_result.is_some());
    }

    #[test]
    fn test_deactivate_and_revoke() {
        let operator_id = Uuid::now_v7();

        // Start with an active onboarding
        let connectors = default_connectors();
        let schema = connectors.iter().find(|c| c.connector_id == "network_international").unwrap();
        let mut credentials = HashMap::new();
        credentials.insert("merchant_id".into(), "MER-12345".into());
        credentials.insert("api_key".into(), "abc123def456abc123def456abc12345".into());
        credentials.insert("environment".into(), "sandbox".into());

        let mut request = OnboardingRequest::new(
            operator_id,
            "network_international".into(),
            "Test".into(),
            "sandbox".into(),
        );
        request.submit_credentials(credentials, schema).unwrap();
        request.start_test().unwrap();
        request.record_test_success(ConnectionTestResult {
            success: true,
            latency_ms: 100,
            error: None,
            merchant_name: None,
            permissions: vec![],
        }).unwrap();

        assert_eq!(request.status, OnboardingStatus::Active);

        // Deactivate
        assert!(request.deactivate().is_ok());
        assert_eq!(request.status, OnboardingStatus::Deactivated);
        assert_eq!(request.health_status, HealthStatus::Down);

        // Reactivate from Deactivated
        assert!(OnboardingStatus::Deactivated.can_transition_to(&OnboardingStatus::Active));

        // Revoke
        assert!(request.revoke().is_ok());
        assert_eq!(request.status, OnboardingStatus::Revoked);
        assert!(request.status.is_terminal());
    }

    #[test]
    fn test_invalid_state_transitions_return_error() {
        let operator_id = Uuid::now_v7();
        let mut request = OnboardingRequest::new(
            operator_id,
            "network_international".into(),
            "Test".into(),
            "sandbox".into(),
        );

        // Can't test before submitting credentials
        assert!(request.start_test().is_err());

        // Can't record success without testing
        assert!(request.record_test_success(ConnectionTestResult {
            success: true,
            latency_ms: 0,
            error: None,
            merchant_name: None,
            permissions: vec![],
        }).is_err());
    }
}
