//! Security-focused integration tests — verify critical security controls.

#[cfg(test)]
mod security_tests {
    use platform_middleware::auth::{AuthPrincipal, AuthMethod, ApiKeyLookup};
    use platform_middleware::abac::{AbacContext, evaluate_policy};
    use uuid::Uuid;

    // === A01: Broken Access Control Tests ===

    #[test]
    fn test_read_only_cannot_register_operator() {
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "read_only".to_string(),
            action: "create".to_string(),
            resource: "operator".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        assert!(evaluate_policy(&ctx).is_err());
    }

    #[test]
    fn test_api_client_cannot_update_status() {
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "api_client".to_string(),
            action: "update_status".to_string(),
            resource: "operator".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        assert!(evaluate_policy(&ctx).is_err());
    }

    #[test]
    fn test_platform_admin_can_do_anything() {
        let actions = ["create", "read", "update", "delete", "verify", "update_status"];
        let resources = ["operator", "payment", "invoice", "subscription", "dispute"];
        for action in &actions {
            for resource in &resources {
                let ctx = AbacContext {
                    principal_id: Uuid::now_v7(),
                    role: "platform_admin".to_string(),
                    action: action.to_string(),
                    resource: resource.to_string(),
                    resource_id: None,
                    amount: None,
                    ip_address: None,
                    operator_id: None,
                };
                assert!(evaluate_policy(&ctx).is_ok(), "platform_admin should be able to {} on {}", action, resource);
            }
        }
    }

    #[test]
    fn test_compliance_officer_can_review_kyb() {
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "compliance_officer".to_string(),
            action: "review".to_string(),
            resource: "kyb".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        assert!(evaluate_policy(&ctx).is_ok());
    }

    // === A04: Cryptographic Tests ===

    #[test]
    fn test_encryption_roundtrip() {
        let key = [42u8; 32];
        let plaintext = "sensitive-payment-data";
        let encrypted = platform_config::encryption::encrypt_field(plaintext, &key).unwrap();
        let decrypted = platform_config::encryption::decrypt_field(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_different_each_time() {
        let key = [1u8; 32];
        let enc1 = platform_config::encryption::encrypt_field("test", &key).unwrap();
        let enc2 = platform_config::encryption::encrypt_field("test", &key).unwrap();
        assert_ne!(enc1, enc2); // Different nonces
    }

    #[test]
    fn test_encryption_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let encrypted = platform_config::encryption::encrypt_field("secret", &key1).unwrap();
        assert!(platform_config::encryption::decrypt_field(&encrypted, &key2).is_err());
    }

    // === A07: Authentication Tests ===

    #[test]
    fn test_client_fingerprint_deterministic() {
        let fp1 = platform_middleware::client_fingerprint("10.0.0.1", "Mozilla/5.0");
        let fp2 = platform_middleware::client_fingerprint("10.0.0.1", "Mozilla/5.0");
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_client_fingerprint_varies() {
        let fp1 = platform_middleware::client_fingerprint("10.0.0.1", "Mozilla/5.0");
        let fp2 = platform_middleware::client_fingerprint("10.0.0.2", "Mozilla/5.0");
        let fp3 = platform_middleware::client_fingerprint("10.0.0.1", "Chrome/120");
        assert_ne!(fp1, fp2);
        assert_ne!(fp1, fp3);
    }

    // === Payment Status State Machine Tests ===

    #[test]
    fn test_payment_lifecycle_happy_path() {
        use shared_types::{PaymentStatus, PaymentCommand};
        assert!(PaymentStatus::Created.can_transition(&PaymentCommand::Authorize));
        assert!(PaymentStatus::Authorizing.can_transition(&PaymentCommand::Authorize));
        assert!(PaymentStatus::Authorized.can_transition(&PaymentCommand::Capture));
        assert!(PaymentStatus::Capturing.can_transition(&PaymentCommand::Capture));
        assert!(PaymentStatus::Captured.can_transition(&PaymentCommand::Refund));
    }

    #[test]
    fn test_cannot_double_authorize() {
        use shared_types::{PaymentStatus, PaymentCommand};
        assert!(!PaymentStatus::Authorized.can_transition(&PaymentCommand::Authorize));
    }

    #[test]
    fn test_cannot_capture_before_authorize() {
        use shared_types::{PaymentStatus, PaymentCommand};
        assert!(!PaymentStatus::Created.can_transition(&PaymentCommand::Capture));
    }

    // === Money Precision Tests ===

    #[test]
    fn test_money_bhd_precision() {
        use shared_types::{Money, CurrencyCode};
        let money = Money { amount_minor_units: 1234, currency: CurrencyCode::new("BHD").unwrap() };
        assert_eq!(money.currency.minor_unit_precision(), 3);
    }

    #[test]
    fn test_money_jpy_precision() {
        use shared_types::{Money, CurrencyCode};
        let money = Money { amount_minor_units: 1000, currency: CurrencyCode::new("JPY").unwrap() };
        assert_eq!(money.currency.minor_unit_precision(), 0);
    }

    // === Circuit Breaker Tests ===

    #[tokio::test]
    async fn test_circuit_breaker_trip_and_recovery() {
        use std::time::Duration;
        // This is tested in connector-gateway's own tests
        // This is a placeholder for integration-level testing
    }

    // === API Key Lookup Trait Tests ===

    struct MockApiKeyLookup;

    #[async_trait::async_trait]
    impl ApiKeyLookup for MockApiKeyLookup {
        async fn find_principal_by_key_hash(&self, _key_hash: &[u8]) -> Result<Option<(Uuid, String)>, String> {
            Ok(Some((Uuid::now_v7(), "api_client".to_string())))
        }
    }

    #[tokio::test]
    async fn test_api_key_lookup_trait() {
        let lookup = MockApiKeyLookup;
        let result = lookup.find_principal_by_key_hash(b"test").await.unwrap();
        assert!(result.is_some());
        let (pid, role) = result.unwrap();
        assert!(!pid.is_nil());
        assert_eq!(role, "api_client");
    }

    // === Decline Reason Tests ===

    #[test]
    fn test_retryable_declines() {
        use shared_types::{DeclineReason, FailoverConfig};
        let config = FailoverConfig::default();
        assert!(DeclineReason::InsufficientFunds.is_retryable(&config));
        assert!(DeclineReason::IssuerUnavailable.is_retryable(&config));
        assert!(!DeclineReason::SuspectedFraud.is_retryable(&config));
        assert!(!DeclineReason::InvalidCard.is_retryable(&config));
    }
}
