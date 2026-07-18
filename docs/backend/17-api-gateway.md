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

---

## 4. Security Headers

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
