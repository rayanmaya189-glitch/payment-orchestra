# 17 — api-gateway (Cross-Cutting)

Single ingress for all external REST/gRPC-Web traffic.

---

## 1. Responsibilities

- **GW-001**: Single ingress point
- **GW-002**: TLS termination, JWT/API-key validation, actor context extraction
- **GW-003**: Per-endpoint rate limiting (Redis sliding window)
- **GW-004**: Request routing to backend services
- **GW-005**: Webhook signature verification NOT here (per-connector, Part 7)

---

## 2. Authentication Flow

```
Request → TLS termination → Extract auth header
  → If JWT: validate signature, expiry, aud claim → extract principal_id
  → If API key: hash with Argon2id → lookup in DB → extract scopes
  → Set actor_context in gRPC metadata
  → Forward to backend service
```

---

## 3. Rate Limiting

```rust
pub struct RateLimitConfig {
    pub endpoint: String,
    pub window_seconds: u32,
    pub max_requests: u32,
    pub per: RateLimitPer, // Ip | ApiKey | Tenant
}
```

| Endpoint | Limit | Window |
|---|---|---|
| `/v1/payment-intents` (checkout) | 1000 | 60s |
| `/v1/invoices` | 100 | 60s |
| `/v1/auth/login` | 10 | 60s (per IP) |
| `/v1/assistant/query` | 30 | 60s (separate quota) |
| `/v1/operators` | 100 | 60s |
| `/v1/kyb-cases` | 10 | 60s |
| `/v1/settlements` | 50 | 60s |
| `/v1/disputes` | 50 | 60s |
| `/v1/subscriptions` | 50 | 60s |
| `/v1/risk/assess` | 100 | 60s |
| `/v1/documents` | 20 | 60s |
| `/v1/analytics/*` | 100 | 60s |
| `/v1/webhooks` | 10 | 60s |

**Concurrent Request Limiting** (RL-CONC-001):

```rust
// Redis-backed per API key: max 100 concurrent in-flight requests
// TTL safety net: 30s auto-decrement for crashed connections
```

---

## 4. CORS Policy (CORS-001)

```rust
pub fn cors_middleware(allowed_origins: &[String]) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowedOrigins::list(allowed_origins)) // exact match only
        .allow_methods([GET, POST, PUT, PATCH, DELETE, OPTIONS])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, X_API_KEY, X_IDEMPOTENCY_KEY, X_REQUEST_ID, X_CSRF_TOKEN])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}
```

---

## 5. Request Size & Timeout Limits (APISEC-001/002)

```rust
pub struct RequestLimits {
    pub max_standard_body: usize,      // 1MB
    pub max_document_upload: usize,    // 10MB
    pub max_payment_creation: usize,   // 100KB
    pub general_timeout: Duration,     // 30s
    pub checkout_timeout: Duration,    // 10s
}
```

---

## 6. Input Validation (APISEC-003)

All API inputs validated against OpenAPI/gRPC schema at API Gateway before reaching domain services:
- Type validation (string, integer, enum)
- Length/range validation (min/max, string length)
- Format validation (UUIDv7, ISO 4217, ISO 8601 with 3-digit ms)
- Required field validation

---

## 7. Request Correlation (REQ-003)

Every inbound request generates UUIDv7 `request_id`. Propagated in:
- gRPC metadata: `x-request-id`
- HTTP header: `X-Request-ID`
- Log context
- Distributed trace
- Error responses

---

## 8. Security Headers (HDR-001)

```
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Content-Security-Policy: default-src 'self'; script-src 'self'; ...
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()
X-XSS-Protection: 0
```

---

## 5. TDD Tests

```rust
#[tokio::test]
async fn test_valid_jwt_authenticated() {
    let response = gateway.handle(request_with_valid_jwt).await;
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_expired_jwt_rejected() {
    let response = gateway.handle(request_with_expired_jwt).await;
    assert_eq!(response.status, 401);
}

#[tokio::test]
async fn test_rate_limit_enforced() {
    // Send 1001 requests in 60 seconds to checkout endpoint
    // 1001st should get 429
}

#[tokio::test]
async fn test_security_headers_present() {
    let response = gateway.handle(any_request).await;
    assert!(response.headers.contains_key("strict-transport-security"));
    assert!(response.headers.contains_key("x-content-type-options"));
}
```
