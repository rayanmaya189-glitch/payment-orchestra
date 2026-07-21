# 17 — api-gateway (Cross-Cutting)

Single ingress for all external protobuf-over-HTTP traffic. **No REST, no GET, no path variables, no query strings.**

---

## 1. API Design Philosophy — Strict Protobuf

**PROTO-001**: All external API requests and responses use Protocol Buffers (protobuf) as the serialization format. Requests are sent as `POST` with `Content-Type: application/protobuf`.

**PROTO-002**: No REST conventions — no GET/PUT/PATCH/DELETE, no path variables, no query strings. Every operation is a `POST` to a service-specific endpoint with a protobuf request body.

**PROTO-003**: Endpoints follow the pattern: `POST /orchestration.v1.OrchestrationService/CreatePaymentIntent`

**PROTO-004**: The API Gateway translates external protobuf-over-HTTP to internal gRPC (same protobuf schema, native gRPC transport between services).

### Why Protobuf-Only

- **Type safety**: Request/response schemas enforced at wire level, not by documentation
- **Code generation**: SDK generation from `.proto` files for all client languages
- **Consistency**: Same schema for external API, internal gRPC, and event payloads
- **Performance**: Binary serialization is smaller and faster than JSON
- **Evolution**: Additive field changes are backward-compatible by design

---

## 2. Endpoint Structure

### Service-Based Routing

```
POST /{package}.{ServiceName}/{MethodName}
Content-Type: application/protobuf
Authorization: Bearer <JWT> | X-Api-Key: <key>

<protobuf binary request body>
```

**Response**: `<protobuf binary response body>` with `Content-Type: application/protobuf`

### Examples

```
# Create payment intent
POST /orchestration.v1.OrchestrationService/CreatePaymentIntent
Content-Type: application/protobuf
Authorization: Bearer eyJ...

<CreatePaymentIntentRequest protobuf>

# Get payment intent (no path variable — ID is in request body)
POST /orchestration.v1.OrchestrationService/GetPaymentIntent
Content-Type: application/protobuf

<GetPaymentIntentRequest protobuf>

# List payments (no query string — filters in request body)
POST /orchestration.v1.OrchestrationService/ListPaymentIntents
Content-Type: application/protobuf

<ListPaymentIntentsRequest protobuf>
```

---

## 3. Responsibilities

- **GW-001**: Single ingress point for all external protobuf-over-HTTP traffic
- **GW-002**: TLS termination, JWT/API-key validation, actor context extraction
- **GW-003**: Per-service rate limiting (Redis sliding window)
- **GW-004**: Request routing to backend services (protobuf → gRPC translation)
- **GW-005**: Webhook signature verification NOT here (per-connector, Part 7)
- **GW-006**: Protobuf schema validation before forwarding to backend services
- **GW-007**: Request/response logging for audit trail (BIZ-040)

---

## 4. Authentication Flow

```
Request → TLS termination → Extract auth header
  → If JWT: validate signature, expiry, aud claim → extract principal_id
  → If API key: hash with Argon2id → lookup in DB → extract scopes
  → Validate protobuf schema (reject malformed requests before forwarding)
  → Set actor_context in gRPC metadata
  → Forward to backend service via native gRPC
```

---

## 5. Rate Limiting

```rust
pub struct RateLimitConfig {
    pub service: String,        // e.g., "orchestration.v1.OrchestrationService"
    pub method: String,         // e.g., "CreatePaymentIntent"
    pub window_seconds: u32,
    pub max_requests: u32,
    pub per: RateLimitPer,      // Ip | ApiKey | Tenant
}
```

| Service.Method | Limit | Window |
|---|---|---|
| `OrchestrationService/CreatePaymentIntent` | 1000 | 60s |
| `OrchestrationService/AuthorizePaymentIntent` | 1000 | 60s |
| `OrchestrationService/CapturePaymentIntent` | 500 | 60s |
| `OrchestrationService/RefundPaymentIntent` | 200 | 60s |
| `InvoiceService/CreateInvoice` | 50 | 60s |
| `SubscriptionService/CreateSubscription` | 50 | 60s |
| `IAMService/Authenticate` | 10 | 60s (per IP) |
| `AIAssistantService/AskQuestion` | 30 | 60s |
| `ComplianceService/SubmitKybEvidence` | 10 | 60s |
| `AnalyticsService/GetDashboard` | 100 | 60s |
| `WebhookService/CreateSubscription` | 10 | 60s |

**Concurrent Request Limiting** (RL-CONC-001):

```rust
// Redis-backed per API key: max 100 concurrent in-flight requests
// TTL safety net: 30s auto-decrement for crashed connections
```

---

## 6. CORS Policy (CORS-001)

```rust
pub fn cors_middleware(allowed_origins: &[String]) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowedOrigins::list(allowed_origins))
        .allow_methods([POST, OPTIONS])        // Only POST — no GET/PUT/PATCH/DELETE
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, X_API_KEY, X_IDEMPOTENCY_KEY, X_REQUEST_ID, X_CSRF_TOKEN])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}
```

---

## 7. Request Size & Timeout Limits

```rust
pub struct RequestLimits {
    pub max_protobuf_body: usize,       // 1MB (standard requests)
    pub max_document_upload: usize,     // 10MB (document upload via streaming)
    pub max_payment_creation: usize,    // 100KB
    pub general_timeout: Duration,      // 30s
    pub checkout_timeout: Duration,     // 10s
}
```

---

## 8. Input Validation

All API inputs validated against protobuf schema at API Gateway before reaching domain services:

- **Schema validation**: Protobuf decoder validates field types, required fields, enum values
- **Length/range validation**: Field-level constraints enforced in `.proto` definitions
- **Format validation**: UUIDv7, ISO 4217, ISO 8601 with 3-digit ms precision
- **Semantic validation**: Domain-specific rules enforced in command handlers (not at gateway)

---

## 9. Request Correlation (REQ-003)

Every inbound request generates UUIDv7 `request_id`. Propagated in:
- gRPC metadata: `x-request-id`
- HTTP header: `X-Request-ID`
- Log context
- Distributed trace
- Error responses (in protobuf error detail)

---

## 10. Security Headers (HDR-001)

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

## 11. Error Handling — Protobuf Error Detail

```protobuf
syntax = "proto3";
package common.v1;

message ErrorDetail {
  string code = 1;              // machine-readable enum (e.g., "INSUFFICIENT_REFUNDABLE_BALANCE")
  string message = 2;           // human-readable (may change wording)
  string request_id = 3;        // correlation ID
  map<string, string> details = 4; // additional context (key-value pairs)
}
```

**PROTO-ERR-001**: Errors are returned as protobuf messages with HTTP status 200 + error detail in response body, OR as HTTP 4xx/5xx with `ErrorDetail` in body. The platform uses HTTP status codes for transport-level errors only (401, 403, 429, 503); business errors always use HTTP 200 with `ErrorDetail`.

**PROTO-ERR-002**: Error codes are stable, documented enums (never free-text strings merchants are expected to parse).

---

## 12. Protobuf Schema Evolution Rules

- **PROTO-EVOLVE-001**: New fields are additive within a schema version (backward-compatible)
- **PROTO-EVOLVE-002**: Breaking changes increment package version (e.g., `orchestration.v1` → `orchestration.v2`)
- **PROTO-EVOLVE-003**: During version transition, API Gateway supports both versions for deprecation window (default: 12 months)
- **PROTO-EVOLVE-004**: Deprecated fields are marked with `deprecated = true` option, not removed

---

## 13. TDD Tests

```rust
#[tokio::test]
async fn test_protobuf_request_authenticated() {
    let request = CreatePaymentIntentRequest { /* ... */ };
    let response = gateway.handle_protobuf(
        "/orchestration.v1.OrchestrationService/CreatePaymentIntent",
        request.encode_to_vec(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_expired_jwt_rejected() {
    let response = gateway.handle_protobuf(
        "/orchestration.v1.OrchestrationService/CreatePaymentIntent",
        request.encode_to_vec(),
        Some(expired_jwt),
    ).await;
    assert_eq!(response.status, 401);
}

#[tokio::test]
async fn test_malformed_protobuf_rejected() {
    let response = gateway.handle_protobuf(
        "/orchestration.v1.OrchestrationService/CreatePaymentIntent",
        b"not protobuf".to_vec(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 400); // bad request
}

#[tokio::test]
async fn test_rate_limit_enforced() {
    // Send 1001 requests in 60 seconds to CreatePaymentIntent
    // 1001st should get 429
}

#[tokio::test]
async fn test_security_headers_present() {
    let response = gateway.handle(any_request).await;
    assert!(response.headers.contains_key("strict-transport-security"));
    assert!(response.headers.contains_key("x-content-type-options"));
}

#[tokio::test]
async fn test_no_get_endpoints_accepted() {
    // Any GET request should return 405 Method Not Allowed
    let response = gateway.handle(get_request("/orchestration.v1.OrchestrationService/GetPaymentIntent")).await;
    assert_eq!(response.status, 405);
}

#[tokio::test]
async fn test_no_path_variables_accepted() {
    // Any request with path variables should return 404
    let response = gateway.handle(post_request("/v1/payment-intents/some-id")).await;
    assert_eq!(response.status, 404);
}
```
