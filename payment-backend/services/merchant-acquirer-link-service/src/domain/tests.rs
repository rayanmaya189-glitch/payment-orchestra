#[cfg(test)]
mod tests {
    use crate::domain::*;
    use uuid::Uuid;

    fn create_test_link() -> MerchantAcquirerLink {
        MerchantAcquirerLink::new(
            Uuid::now_v7(),
            "checkout_com".into(),
            "Production Gateway".into(),
            LinkEnvironment::Production,
            vec![1, 2, 3, 4],
            "abc123hash".into(),
        )
    }

    #[test]
    fn test_link_creation() {
        let link = create_test_link();
        assert_eq!(link.status, LinkStatus::Testing);
        assert_eq!(link.health_status, HealthStatus::Unknown);
        assert!(!link.status.is_routable());
    }

    #[test]
    fn test_connection_test_success() {
        let mut link = create_test_link();
        link.record_connection_test(true, 150);
        assert_eq!(link.status, LinkStatus::Active);
        assert_eq!(link.health_status, HealthStatus::Healthy);
        assert!(link.status.is_routable());
        assert!(link.last_tested_at.is_some());
    }

    #[test]
    fn test_connection_test_failure() {
        let mut link = create_test_link();
        link.record_connection_test(false, 0);
        assert_eq!(link.status, LinkStatus::Testing);
        assert_eq!(link.health_status, HealthStatus::Unreachable);
    }

    #[test]
    fn test_disable() {
        let mut link = create_test_link();
        link.record_connection_test(true, 100);
        assert!(link.disable().is_ok());
        assert_eq!(link.status, LinkStatus::Disabled);
    }

    #[test]
    fn test_double_disable_fails() {
        let mut link = create_test_link();
        link.disable().unwrap();
        assert!(link.disable().is_err());
    }

    #[test]
    fn test_enable_from_disabled() {
        let mut link = create_test_link();
        link.disable().unwrap();
        assert!(link.enable().is_ok());
        assert_eq!(link.status, LinkStatus::Testing);
    }

    #[test]
    fn test_rotate_credentials() {
        let mut link = create_test_link();
        link.record_connection_test(true, 100);
        link.rotate_credentials(vec![5, 6, 7, 8], "newhash".into());
        assert_eq!(link.status, LinkStatus::Testing);
        assert_eq!(link.credentials_hash, "newhash");
    }

    #[test]
    fn test_expired_credentials_cannot_enable() {
        let mut link = create_test_link();
        link.mark_credentials_expired();
        assert_eq!(link.status, LinkStatus::CredentialsExpired);
        assert!(link.enable().is_err());
    }

    #[test]
    fn test_credentials_hash_consistent() {
        let hash1 = MerchantAcquirerLink::compute_credentials_hash(r#"{"key":"value"}"#);
        let hash2 = MerchantAcquirerLink::compute_credentials_hash(r#"{"key":"value"}"#);
        assert_eq!(hash1, hash2);
    }
}
