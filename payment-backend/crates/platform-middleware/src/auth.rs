//! Authentication middleware — validates JWT tokens and API keys.
//!
//! JWT tokens are validated using RS256 (AUTH-017) with a PEM-encoded public key.
//! Falls back to HS256 with a shared secret if no public key is configured (dev mode).
//! API keys are validated by prefix lookup and hash comparison.
//!
//! Both methods return an `AuthContext` containing the principal's identity and permissions.

use uuid::Uuid;
use jsonwebtoken::{decode, Validation, Algorithm, DecodingKey};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::iter::FromIterator;
use platform_error::{PlatformError, ValidationError};

use shared_types::auth_context::AuthContext;

// ─── JWT Claims ──────────────────────────────────────────────────────────────

/// JWT payload claims decoded from the token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject — the principal UUID
    pub sub: String,
    /// Principal type: "human" | "api_key" | "service"
    pub pty: String,
    /// Operator ID (optional, for operators acting on behalf of merchants)
    pub oid: Option<String>,
    /// Permissions — list of permission strings
    pub perms: Vec<String>,
    /// Session ID
    pub sid: Option<String>,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiry (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
}

// ─── Token Types ─────────────────────────────────────────────────────────────

/// Types of authentication tokens supported.
pub enum AuthToken {
    /// JWT bearer token (format: "Bearer <token>")
    Bearer(String),
    /// API key (format: "pk_live_xxx..." or "pk_test_xxx...")
    ApiKey(String),
}

/// Result of successful authentication.
pub struct AuthResult {
    pub auth_context: AuthContext,
    pub token_type: &'static str,
}

// ─── JWT Validation ──────────────────────────────────────────────────────────

/// Validate a JWT token and return the decoded claims.
///
/// Uses RS256 if `JWT_PUBLIC_KEY_PEM` env var is set (production, AUTH-017).
/// Falls back to HS256 with `jwt_secret` (dev mode).
/// Validates:
/// - Signature (RS256 with PEM public key, or HS256 with shared secret)
/// - Expiry (`exp` claim)
/// - Issuer (must match `expected_issuer`, if provided)
/// - Audience (must match `expected_audience`, if provided)
pub fn validate_jwt(token: &str, jwt_secret: &str, expected_issuer: Option<&str>, expected_audience: Option<&str>) -> Result<JwtClaims, PlatformError> {
    // Try RS256 with PEM-encoded public key first (production, AUTH-017)
    if let Ok(pem) = std::env::var("JWT_PUBLIC_KEY_PEM") {
        if !pem.is_empty() {
            match DecodingKey::from_rsa_pem(pem.as_bytes()) {
                Ok(key) => {
                    let mut validation = Validation::new(Algorithm::RS256);
                    validation.validate_exp = true;
                    validation.required_spec_claims = HashSet::from_iter(["sub".to_string(), "exp".to_string(), "iat".to_string()]);
                    if let Some(issuer) = expected_issuer { validation.set_issuer(&[issuer]); }
                    if let Some(audience) = expected_audience { validation.set_audience(&[audience]); }

                    return decode::<JwtClaims>(token, &key, &validation)
                        .map(|d| d.claims)
                        .map_err(|e| PlatformError::AuthorizationDenied(format!("JWT RS256 validation failed: {}", e)));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to parse JWT_PUBLIC_KEY_PEM, falling back to HS256");
                }
            }
        }
    }

    // Fallback: HS256 with shared secret (dev mode)
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.required_spec_claims = HashSet::from_iter(["sub".to_string(), "exp".to_string(), "iat".to_string()]);

    if let Some(issuer) = expected_issuer {
        validation.set_issuer(&[issuer]);
    }
    if let Some(audience) = expected_audience {
        validation.set_audience(&[audience]);
    }

    let key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let token_data = decode::<JwtClaims>(token, &key, &validation)
        .map_err(|e| PlatformError::AuthorizationDenied(format!("JWT HS256 validation failed: {}", e)))?;

    Ok(token_data.claims)
}

/// Extract the bearer token from an Authorization header value.
/// Format: "Bearer <token>"
pub fn extract_bearer_token(auth_header: &str) -> Result<String, PlatformError> {
    let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
    if parts.len() != 2 || parts[0] != "Bearer" {
        return Err(PlatformError::AuthorizationDenied("Invalid authorization header format".into()));
    }
    Ok(parts[1].to_string())
}

// ─── API Key Validation ─────────────────────────────────────────────────────

/// Validate an API key against its stored hash.
///
/// API keys follow the format: `<prefix>_<random>` where prefix is e.g. "pk_live".
/// The stored hash uses argon2id for password hashing.
pub fn validate_api_key(raw_key: &str, stored_hash: &[u8]) -> Result<bool, PlatformError> {
    let hash_str = std::str::from_utf8(stored_hash)
        .map_err(|_| PlatformError::Validation(ValidationError::InvalidValue {
            field: "stored_hash".into(),
            reason: "Invalid UTF-8 in stored hash".into(),
        }))?;

    let parsed_hash = PasswordHash::new(hash_str)
        .map_err(|e| PlatformError::Internal(format!("Failed to parse stored hash: {}", e)))?;

    Ok(Argon2::default().verify_password(raw_key.as_bytes(), &parsed_hash).is_ok())
}

/// Extract the API key prefix from a raw key string.
/// E.g., "pk_live_abc123" → "pk_live"
pub fn extract_api_key_prefix(raw_key: &str) -> Option<&str> {
    // Format: <prefix>_<random32chars>
    // Prefix is typically "pk_live" or "pk_test"
    let underscore_pos = raw_key.find('_')?;
    let second_underscore = raw_key[underscore_pos + 1..].find('_')?;
    Some(&raw_key[..=underscore_pos + second_underscore])
}

// ─── Unified Auth ────────────────────────────────────────────────────────────

/// Convert JWT claims into an AuthContext.
pub fn claims_to_auth_context(claims: &JwtClaims) -> AuthContext {
    AuthContext {
        principal_id: Uuid::parse_str(&claims.sub).unwrap_or_default(),
        principal_type: claims.pty.clone(),
        operator_id: claims.oid.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        permissions: claims.perms.clone(),
        session_id: claims.sid.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        ip_address: None,
        user_agent: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, Header, EncodingKey};

    fn test_secret() -> &'static str {
        "test_secret_key_for_unit_tests_only"
    }

    fn create_test_token(claims: JwtClaims) -> String {
        let key = EncodingKey::from_secret(test_secret().as_bytes());
        encode(&Header::new(Algorithm::HS256), &claims, &key).unwrap()
    }

    #[test]
    fn test_extract_bearer_token_valid() {
        let result = extract_bearer_token("Bearer eyJhbGciOiJIUzI1NiJ9.token");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "eyJhbGciOiJIUzI1NiJ9.token");
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        assert!(extract_bearer_token("Basic dXNlcjpwYXNz").is_err());
        assert!(extract_bearer_token("Bearer").is_err());
        assert!(extract_bearer_token("").is_err());
    }

    #[test]
    fn test_validate_jwt_valid_token() {
        let claims = JwtClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".into(),
            pty: "human".into(),
            oid: None,
            perms: vec!["payment_intent:create".into(), "payment_intent:read".into()],
            sid: None,
            iss: "payment-orchestra".into(),
            aud: "api-gateway".into(),
            exp: 9999999999,
            iat: 1000000000,
        };

        let token = create_test_token(claims);
        let result = validate_jwt(&token, test_secret(), Some("payment-orchestra"), Some("api-gateway"));
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.pty, "human");
        assert_eq!(decoded.perms.len(), 2);
    }

    #[test]
    fn test_validate_jwt_expired_token() {
        let claims = JwtClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".into(),
            pty: "human".into(),
            oid: None,
            perms: vec![],
            sid: None,
            iss: "payment-orchestra".into(),
            aud: "api-gateway".into(),
            exp: 1000000000, // expired
            iat: 500000000,
        };

        let token = create_test_token(claims);
        let result = validate_jwt(&token, test_secret(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_jwt_invalid_signature() {
        let claims = JwtClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".into(),
            pty: "human".into(),
            oid: None,
            perms: vec![],
            sid: None,
            iss: "payment-orchestra".into(),
            aud: "api-gateway".into(),
            exp: 9999999999,
            iat: 1000000000,
        };

        let token = create_test_token(claims);
        // Use a different secret for validation
        let result = validate_jwt(&token, "wrong_secret", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_api_key_prefix() {
        assert_eq!(extract_api_key_prefix("pk_live_abc123def456"), Some("pk_live"));
        assert_eq!(extract_api_key_prefix("pk_test_abc123"), Some("pk_test"));
        assert_eq!(extract_api_key_prefix("invalid"), None);
    }

    #[test]
    fn test_claims_to_auth_context() {
        let claims = JwtClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".into(),
            pty: "human".into(),
            oid: Some("660e8400-e29b-41d4-a716-446655440001".into()),
            perms: vec!["admin.*".into()],
            sid: Some("770e8400-e29b-41d4-a716-446655440002".into()),
            iss: "test".into(),
            aud: "test".into(),
            exp: 9999999999,
            iat: 1000000000,
        };

        let ctx = claims_to_auth_context(&claims);
        assert!(ctx.is_admin());
        assert_eq!(ctx.principal_type, "human");
    }
}
