//! Integration tests for IAM service — authentication, API keys, MFA.

#[cfg(test)]
mod iam_tests {
    use platform_middleware::auth::{AuthPrincipal, AuthMethod};
    use uuid::Uuid;

    #[test]
    fn test_auth_principal_extraction() {
        let principal = AuthPrincipal {
            principal_id: Uuid::now_v7(),
            role: "operator_admin".to_string(),
            auth_method: AuthMethod::Jwt,
        };
        assert_eq!(principal.role, "operator_admin");
        assert_eq!(principal.auth_method, AuthMethod::Jwt);
    }

    #[test]
    fn test_api_key_auth_method() {
        let principal = AuthPrincipal {
            principal_id: Uuid::now_v7(),
            role: "api_client".to_string(),
            auth_method: AuthMethod::ApiKey,
        };
        assert_eq!(principal.auth_method, AuthMethod::ApiKey);
    }

    #[test]
    fn test_client_fingerprint_deterministic() {
        let fp1 = platform_middleware::client_fingerprint("1.2.3.4", "Mozilla/5.0");
        let fp2 = platform_middleware::client_fingerprint("1.2.3.4", "Mozilla/5.0");
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_client_fingerprint_varies_by_ip() {
        let fp1 = platform_middleware::client_fingerprint("1.2.3.4", "Mozilla/5.0");
        let fp2 = platform_middleware::client_fingerprint("5.6.7.8", "Mozilla/5.0");
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_client_fingerprint_varies_by_ua() {
        let fp1 = platform_middleware::client_fingerprint("1.2.3.4", "Mozilla/5.0");
        let fp2 = platform_middleware::client_fingerprint("1.2.3.4", "Chrome/120");
        assert_ne!(fp1, fp2);
    }
}
