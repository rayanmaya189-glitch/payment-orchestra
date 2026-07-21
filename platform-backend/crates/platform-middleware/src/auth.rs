//! Authentication middleware — JWT and API key validation.
//!
//! - `JwtAuthLayer`: Validates Bearer tokens from Authorization header.
//! - `ApiKeyAuthLayer`: Validates API keys from X-Api-Key header.
//! - `AuthPrincipal`: Axum extractor that provides the authenticated principal.

use axum::{
    body::Body,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use platform_config::AuthConfig;
use platform_error::PlatformError;

/// Decoded JWT claims — matches IAM service Claims struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub role: String,
    pub jti: String,
    pub iss: String,
    pub aud: String,
}

/// Authenticated principal — extracted from request by middleware.
#[derive(Debug, Clone)]
pub struct AuthPrincipal {
    pub principal_id: Uuid,
    pub role: String,
    pub auth_method: AuthMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
}

/// Axum extractor for AuthPrincipal — use in handler signatures.
impl<S> FromRequestParts<S> for AuthPrincipal
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut http::request::Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<AuthPrincipal>()
            .cloned()
            .ok_or_else(|| (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "error": "Not authenticated",
                    "code": "UNAUTHORIZED"
                })),
            ))
    }
}

// ==================== JWT Middleware ====================

/// Layer that adds JWT authentication to all routes.
///
/// When `require` is true (default), requests without a valid JWT are rejected with 401.
/// When `require` is false, unauthenticated requests pass through (useful for mixed routes).
#[derive(Clone)]
pub struct JwtAuthLayer {
    config: AuthConfig,
    require: bool,
    /// Optional token blocklist for server-side revocation (OWASP A04/A07).
    blocklist: Option<Arc<dyn TokenBlocklist>>,
}

impl JwtAuthLayer {
    pub fn new(config: AuthConfig) -> Self {
        Self { config, require: true, blocklist: None }
    }

    /// Create a layer that does NOT require authentication (pass-through for unauthenticated).
    pub fn optional(config: AuthConfig) -> Self {
        Self { config, require: false, blocklist: None }
    }

    /// Enable token blocklist for server-side revocation.
    pub fn with_blocklist(mut self, blocklist: Arc<dyn TokenBlocklist>) -> Self {
        self.blocklist = Some(blocklist);
        self
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthService {
            inner,
            config: self.config.clone(),
            require: self.require,
            blocklist: self.blocklist.clone(),
        }
    }
}

#[derive(Clone)]
pub struct JwtAuthService<S> {
    inner: S,
    config: AuthConfig,
    require: bool,
    blocklist: Option<Arc<dyn TokenBlocklist>>,
}

impl<S> Service<http::Request<Body>> for JwtAuthService<S>
where
    S: Service<http::Request<Body>, Response = http::Response<Body>> + Send + Clone + 'static,
    S::Future: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let require = self.require;
        let blocklist = self.blocklist.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Extract Bearer token from Authorization header
            let token = req
                .headers()
                .get(AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            match token {
                Some(token_str) => {
                    // SRS AUTH-017: Support both RS256 (production) and HS256 (dev)
                    // Determine decoding key based on config
                    let (key, allowed_algorithms) = if let Some(ref public_pem) = config.jwt_public_key_pem {
                        (
                            DecodingKey::from_rsa_pem(public_pem.as_bytes())
                                .map_err(|e| PlatformError::Internal(format!("RSA key error: {e}")))
                                .ok()
                                .map(|k| k as DecodingKey),
                            vec![Algorithm::RS256],
                        )
                    } else {
                        (
                            Some(DecodingKey::from_secret(config.jwt_secret.as_bytes())),
                            vec![Algorithm::HS256],
                        )
                    };

                    let Some(decoding_key) = key else {
                        return Ok(forbidden_response("Server configuration error"));
                    };

                    let mut validation = Validation::new(allowed_algorithms.first().copied().unwrap_or(Algorithm::HS256));
                    validation.set_issuer(&["payment-orchestra"]);
                    validation.algorithms = allowed_algorithms;

                    match decode::<Claims>(
                        token_str,
                        &decoding_key,
                        &validation,
                    ) {
                        Ok(token_data) => {
                            let claims = token_data.claims;

                            // Check expiry
                            if claims.exp < Utc::now().timestamp() as usize {
                                return Ok(forbidden_response("Token expired"));
                            }

                            // Check token blocklist for server-side revocation (OWASP A04/A07)
                            if let Some(ref blocklist) = blocklist {
                                if let Ok(true) = blocklist.is_blocked(&claims.jti).await {
                                    return Ok(forbidden_response("Token has been revoked"));
                                }
                            }

                            let principal_id = match Uuid::parse_str(&claims.sub) {
                                Ok(id) => id,
                                Err(_) => return Ok(forbidden_response("Invalid principal ID")),
                            };

                            let principal = AuthPrincipal {
                                principal_id,
                                role: claims.role,
                                auth_method: AuthMethod::Jwt,
                            };

                            req.extensions_mut().insert(principal);
                            inner.call(req).await
                        }
                        Err(_) => Ok(forbidden_response("Invalid token")),
                    }
                }
                None => {
                    // No Authorization header
                    if require {
                        // Authentication required — reject
                        Ok(forbidden_response("Authentication required"))
                    } else {
                        // Optional auth — pass through
                        inner.call(req).await
                    }
                }
            }
        })
    }
}

// ==================== API Key Middleware ====================

/// Trait for API key lookup — implemented by the IAM service's repository.
/// The middleware calls this to validate API keys against the database.
#[async_trait::async_trait]
pub trait ApiKeyLookup: Send + Sync {
    /// Look up a principal by API key hash. Returns (principal_id, role) if valid.
    async fn find_principal_by_key_hash(&self, key_hash: &[u8]) -> Result<Option<(Uuid, String)>, String>;
}

/// Layer that validates API keys from X-Api-Key header.
#[derive(Clone)]
pub struct ApiKeyAuthLayer {
    lookup: Arc<dyn ApiKeyLookup>,
}

impl ApiKeyAuthLayer {
    pub fn new(lookup: Arc<dyn ApiKeyLookup>) -> Self {
        Self { lookup }
    }
}

impl<S> Layer<S> for ApiKeyAuthLayer {
    type Service = ApiKeyAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ApiKeyAuthService {
            inner,
            lookup: self.lookup.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ApiKeyAuthService<S> {
    inner: S,
    lookup: Arc<dyn ApiKeyLookup>,
}

impl<S> Service<http::Request<Body>> for ApiKeyAuthService<S>
where
    S: Service<http::Request<Body>, Response = http::Response<Body>> + Send + Clone + 'static,
    S::Future: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let lookup = self.lookup.clone();

        Box::pin(async move {
            // Check if JWT already authenticated
            if req.extensions().get::<AuthPrincipal>().is_some() {
                return inner.call(req).await;
            }

            // Extract API key from X-Api-Key header
            if let Some(api_key) = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()) {
                // Validate key format (production pk_ prefix)
                if !api_key.starts_with("pk_") || api_key.len() < 20 {
                    return Ok(forbidden_response("Invalid API key format"));
                }

                // Hash the API key with SHA-256 (matching the storage format)
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(api_key.as_bytes());
                let key_hash = hasher.finalize().to_vec();

                // Look up the key in the database
                match lookup.find_principal_by_key_hash(&key_hash).await {
                    Ok(Some((principal_id, role))) => {
                        let principal = AuthPrincipal {
                            principal_id,
                            role,
                            auth_method: AuthMethod::ApiKey,
                        };
                        req.extensions_mut().insert(principal);
                    }
                    Ok(None) => {
                        return Ok(forbidden_response("Invalid API key"));
                    }
                    Err(e) => {
                        tracing::error!("API key lookup failed: {e}");
                        return Ok(forbidden_response("Authentication error"));
                    }
                }
            }

            inner.call(req).await
        })
    }
}

/// Compute a SHA-256 fingerprint from IP + User-Agent for token theft detection.
/// Used by IAM service for session binding (SRS SESS-SEC-003).
pub fn client_fingerprint(ip: &str, user_agent: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(format!("{}|{}", ip, user_agent).as_bytes());
    hex::encode(hasher.finalize())
}

/// Helper to create a 401 JSON response.
fn forbidden_response(message: &str) -> http::Response<Body> {
    http::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::json!({"error": message, "code": "UNAUTHORIZED"}).to_string()))
        .unwrap()
}

// ==================== Token Blocklist (Server-Side Revocation) ====================

/// Trait for token blocklist — allows immediate revocation of JWT tokens.
///
/// JWT tokens are stateless by design, so revoked tokens remain valid until expiry.
/// The blocklist stores revoked JTIs (JWT IDs) with TTL matching the token expiry.
/// This provides immediate revocation for logout, password changes, and security events.
#[async_trait::async_trait]
pub trait TokenBlocklist: Send + Sync {
    /// Add a JTI to the blocklist with TTL matching token expiry.
    async fn block_token(&self, jti: &str, ttl_secs: u64) -> Result<(), String>;

    /// Check if a JTI is in the blocklist (revoked).
    async fn is_blocked(&self, jti: &str) -> Result<bool, String>;
}

/// In-memory token blocklist for testing (not for production).
pub struct InMemoryTokenBlocklist {
    blocked: std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
}

impl InMemoryTokenBlocklist {
    pub fn new() -> Self {
        Self {
            blocked: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl TokenBlocklist for InMemoryTokenBlocklist {
    async fn block_token(&self, jti: &str, ttl_secs: u64) -> Result<(), String> {
        let mut blocked = self.blocked.lock().map_err(|e| e.to_string())?;
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(ttl_secs);
        blocked.insert(jti.to_string(), expires_at);
        Ok(())
    }

    async fn is_blocked(&self, jti: &str) -> Result<bool, String> {
        let blocked = self.blocked.lock().map_err(|e| e.to_string())?;
        match blocked.get(jti) {
            Some(expires_at) => Ok(std::time::Instant::now() < *expires_at),
            None => Ok(false),
        }
    }
}

/// Redis-backed token blocklist for production use.
///
/// Uses Redis SET with TTL for automatic expiry. Each blocked JTI is stored
/// as `blocklist:{jti}` with the remaining TTL as the Redis key expiry.
pub struct RedisTokenBlocklist {
    redis: redis::aio::ConnectionManager,
}

impl RedisTokenBlocklist {
    pub fn new(redis: redis::aio::ConnectionManager) -> Self {
        Self { redis }
    }
}

#[async_trait::async_trait]
impl TokenBlocklist for RedisTokenBlocklist {
    async fn block_token(&self, jti: &str, ttl_secs: u64) -> Result<(), String> {
        let mut redis = self.redis.clone();
        redis::cmd("SET")
            .arg(format!("blocklist:{}", jti))
            .arg("1")
            .arg("EX")
            .arg(ttl_secs)
            .query_async::<()>(&mut redis)
            .await
            .map_err(|e| format!("Redis SET failed: {e}"))?;
        Ok(())
    }

    async fn is_blocked(&self, jti: &str) -> Result<bool, String> {
        let mut redis = self.redis.clone();
        let exists: bool = redis::cmd("EXISTS")
            .arg(format!("blocklist:{}", jti))
            .query_async(&mut redis)
            .await
            .map_err(|e| format!("Redis EXISTS failed: {e}"))?;
        Ok(exists)
    }
}

/// Check if a token's JTI is blocked before accepting it.
///
/// Call this after decoding the JWT to verify the token hasn't been revoked.
pub async fn verify_token_not_blocked(
    claims: &Claims,
    blocklist: &dyn TokenBlocklist,
) -> Result<(), PlatformError> {
    if blocklist.is_blocked(&claims.jti).await.unwrap_or(false) {
        return Err(PlatformError::AuthorizationDenied(
            "Token has been revoked".to_string()
        ));
    }
    Ok(())
}
