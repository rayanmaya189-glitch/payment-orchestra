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
use std::task::{Context, Poll};
use tower::{Layer, Service};

use platform_config::AuthConfig;

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
}

impl JwtAuthLayer {
    pub fn new(config: AuthConfig) -> Self {
        Self { config, require: true }
    }

    /// Create a layer that does NOT require authentication (pass-through for unauthenticated).
    pub fn optional(config: AuthConfig) -> Self {
        Self { config, require: false }
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthService {
            inner,
            config: self.config.clone(),
            require: self.require,
        }
    }
}

#[derive(Clone)]
pub struct JwtAuthService<S> {
    inner: S,
    config: AuthConfig,
    require: bool,
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
                    // Validate JWT
                    let mut validation = Validation::new(Algorithm::HS256);
                    validation.set_issuer(&["payment-orchestra"]);

                    match decode::<Claims>(
                        token_str,
                        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
                        &validation,
                    ) {
                        Ok(token_data) => {
                            let claims = token_data.claims;

                            // Check expiry
                            if claims.exp < Utc::now().timestamp() as usize {
                                return Ok(forbidden_response("Token expired"));
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

/// Layer that validates API keys from X-Api-Key header.
#[derive(Clone)]
pub struct ApiKeyAuthLayer;

impl ApiKeyAuthLayer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiKeyAuthLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for ApiKeyAuthLayer {
    type Service = ApiKeyAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ApiKeyAuthService { inner }
    }
}

#[derive(Clone)]
pub struct ApiKeyAuthService<S> {
    inner: S,
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

                // TODO: In production, hash with Argon2id and lookup in DB.
                // Currently derives a deterministic principal_id for framework correctness.
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(api_key.as_bytes());
                let hash = hasher.finalize();
                let mut uuid_bytes = [0u8; 16];
                uuid_bytes.copy_from_slice(&hash[..16]);
                let principal_id = Uuid::from_bytes(uuid_bytes);

                let principal = AuthPrincipal {
                    principal_id,
                    role: "api_client".to_string(),
                    auth_method: AuthMethod::ApiKey,
                };

                req.extensions_mut().insert(principal);
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
