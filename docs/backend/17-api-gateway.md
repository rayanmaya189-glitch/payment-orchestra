# 17 — api-gateway (Cross-Cutting)

Single ingress for all external traffic. **Dual API**: RESTful JSON for merchant adoption + Protobuf-over-HTTP for performance-sensitive use cases. Internal gRPC for service-to-service.

---

## 1. API Design Philosophy — Dual API Strategy

The platform exposes two API styles for merchants:

| API Style | Content Type | Audience | Priority |
|---|---|---|---|
| **RESTful JSON** | `application/json` | Primary merchant integration | Primary |
| **Protobuf-over-HTTP** | `application/protobuf` | Performance-sensitive/high-volume | Secondary |
| **gRPC** (internal) | `application/grpc` | Service-to-service only | Internal |

### Why REST + Protobuf (Not Protobuf-Only)

Every major payment gateway uses RESTful JSON as their primary API (Stripe, Adyen, Checkout.com, PayPal). The decision to support both is driven by:

- **Merchant adoption**: REST JSON has zero compilation requirements, works with any HTTP client (curl, Postman), is human-readable, and doesn't require protobuf tooling
- **Developer experience**: JSON is familiar to ALL developers across all languages and frameworks
- **SDK simplicity**: Language SDKs are significantly simpler to build and maintain for JSON APIs
- **Performance option**: Protobuf is available as an opt-in for high-volume merchants who need the binary serialization performance

**REST JSON is the recommended default. Protobuf is the power-user option.**

---

## 2. Endpoint Structure

### 2.1 RESTful JSON Endpoints (Primary)

Conventional RESTful paths with path variables and JSON bodies:

```
POST   /v1/payment-intents              # CreatePaymentIntent
POST   /v1/payment-intents/:id/authorize # AuthorizePaymentIntent
POST   /v1/payment-intents/:id/capture  # CapturePaymentIntent
POST   /v1/payment-intents/:id/void     # VoidPaymentIntent
POST   /v1/payment-intents/:id/refund   # RefundPaymentIntent
GET    /v1/payment-intents/:id          # GetPaymentIntent
GET    /v1/payment-intents              # ListPaymentIntents (with query params)
```

**Example Request:**
```bash
curl -X POST https://api.paymentorchestra.com/v1/payment-intents \
  -H "Authorization: Bearer sk_live_abc123" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: unique-key-001" \
  -d '{
    "amount": 10000,
    "currency": "AED",
    "payment_method": {
      "type": "card",
      "card": {
        "number": "4242424242424242",
        "exp_month": 12,
        "exp_year": 2028,
        "cvc": "123"
      }
    }
  }'
```

### 2.2 Protobuf-over-HTTP Endpoints (Secondary)

Same gRPC-style POST for protobuf clients:

```
POST /proto/{package}.{ServiceName}/{MethodName}
Content-Type: application/protobuf

<protobuf binary request body>
```

**Example:**
```
POST /proto/orchestration.v1.OrchestrationService/CreatePaymentIntent
Content-Type: application/protobuf
Authorization: Bearer eyJ...

<CreatePaymentIntentRequest protobuf>
```

### 2.3 API Versioning

- **API-GW-VERSION-001**: REST endpoints are versioned by URL prefix (`/v1/`, `/v2/`)
- **API-GW-VERSION-002**: Protobuf endpoints are versioned by package name (`orchestration.v1`, `orchestration.v2`)
- **API-GW-VERSION-003**: During version transition, both versions are supported for a deprecation window (default: 12 months)
- **API-GW-VERSION-004**: Deprecated versions receive `Sunset` header (RFC 8594) indicating removal date

---

## 3. Responsibilities

- **GW-001**: Single ingress point for all external REST JSON + Protobuf traffic
- **GW-002**: TLS termination, JWT/API-key validation, actor context extraction
- **GW-003**: Per-endpoint rate limiting (Redis sliding window)
- **GW-004**: Request routing — REST JSON → internal domain logic; Protobuf → internal gRPC
- **GW-005**: Webhook signature verification NOT here (per-connector, Part 7)
- **GW-006**: Input validation at gateway level (schema, format, length)
- **GW-007**: Request/response logging for audit trail (BIZ-040)
- **GW-008**: Content negotiation — detect Content-Type and route to appropriate handler
- **GW-009**: Input sanitization and XSS prevention for all request data
- **GW-010**: CORS enforcement with configurable allowed origins

---

## 4. Authentication Flow

```
Request → TLS termination → Extract auth header
  → If JWT: validate signature, expiry, aud claim → extract principal_id
  → If API key: hash with Argon2id → lookup in DB → extract scopes
  → Determine content type (JSON or Protobuf)
  → For REST JSON: parse JSON body, validate against schema
  → For Protobuf: decode protobuf, validate field constraints
  → Set actor_context in request metadata
  → Forward to backend service
```

### API Key Conventions

```
# REST JSON: Bearer token or X-Api-Key header
Authorization: Bearer sk_live_abc123def456
# or
X-Api-Key: sk_live_abc123def456

# API Key Prefix Convention
sk_live_*   = Secret key (live, full access)
pk_live_*   = Publishable key (live, restricted)
sk_test_*   = Secret key (sandbox, full access)
pk_test_*   = Publishable key (sandbox, restricted)
```

---

## 5. Rate Limiting

```rust
pub struct RateLimitConfig {
    pub route: String,              // e.g., "POST /v1/payment-intents" or "orchestration.v1.OrchestrationService"
    pub window_seconds: u32,
    pub max_requests: u32,
    pub per: RateLimitPer,          // Ip | ApiKey | Tenant
}
```

| Route | Limit | Window | Per |
|---|---|---|---|
| `POST /v1/payment-intents` | 1000 | 60s | ApiKey |
| `POST /v1/payment-intents/:id/authorize` | 1000 | 60s | ApiKey |
| `POST /v1/payment-intents/:id/capture` | 500 | 60s | ApiKey |
| `POST /v1/payment-intents/:id/refund` | 200 | 60s | ApiKey |
| `GET /v1/payment-intents` | 500 | 60s | ApiKey |
| `post /v1/invoices` | 50 | 60s | ApiKey |
| `POST /v1/subscriptions` | 50 | 60s | ApiKey |
| `POST /v1/merchant-links` | 20 | 60s | ApiKey |
| `POST /v1/authenticate` | 10 | 60s | IP |
| `POST /v1/ai/ask` | 30 | 60s | ApiKey |
| `POST /v1/compliance/kyb` | 10 | 60s | ApiKey |
| `POST /v1/webhook-endpoints` | 10 | 60s | ApiKey |
| `GET /v1/analytics/*` | 100 | 60s | ApiKey |

---

## 6. CORS Policy (CORS-001)

```rust
pub fn cors_middleware(allowed_origins: &[String]) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowedOrigins::list(allowed_origins))
        .allow_methods([GET, POST, DELETE, PATCH, OPTIONS]) // All REST methods
        .allow_headers([
            CONTENT_TYPE, AUTHORIZATION, X_API_KEY, X_IDEMPOTENCY_KEY,
            X_REQUEST_ID, X_CSRF_TOKEN, X_WEBHOOK_SIGNATURE
        ])
        .expose_headers([X_REQUEST_ID, X_RATE_LIMIT_REMAINING, X_RATE_LIMIT_RESET])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}
```

---

## 7. REST JSON → Internal Translation

The api-gateway translates REST JSON requests to internal operations:

```rust
/// Translation registry: REST route → internal handler
pub struct RestRouter {
    routes: HashMap<RoutePattern, Box<dyn RestHandler>>,
}

impl RestRouter {
    pub fn register_routes(&mut self) {
        // Payment Intents
        self.register(RoutePattern { method: "POST", path: "/v1/payment-intents" }, PaymentIntentCreateHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/payment-intents/:id" }, PaymentIntentGetHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/payment-intents" }, PaymentIntentListHandler);

        // Merchant Links (BYOK)
        self.register(RoutePattern { method: "POST", path: "/v1/merchant-links" }, MerchantLinkCreateHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/merchant-links" }, MerchantLinkListHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/merchant-links/:id" }, MerchantLinkGetHandler);
        self.register(RoutePattern { method: "POST", path: "/v1/merchant-links/:id/test" }, MerchantLinkTestHandler);
        self.register(RoutePattern { method: "DELETE", path: "/v1/merchant-links/:id" }, MerchantLinkDisableHandler);

        // Connectors (BYOK discovery)
        self.register(RoutePattern { method: "GET", path: "/v1/connectors" }, ConnectorListHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/connectors/:id/schema" }, ConnectorSchemaHandler);

        // Routing
        self.register(RoutePattern { method: "POST", path: "/v1/routing-policies" }, RoutingPolicyCreateHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/routing-policies" }, RoutingPolicyListHandler);

        // Webhooks (outbound)
        self.register(RoutePattern { method: "POST", path: "/v1/webhook-endpoints" }, WebhookEndpointCreateHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/webhook-endpoints" }, WebhookEndpointListHandler);
        self.register(RoutePattern { method: "POST", path: "/v1/webhook-endpoints/:id/test" }, WebhookEndpointTestHandler);

        // Analytics
        self.register(RoutePattern { method: "GET", path: "/v1/analytics/transactions" }, AnalyticsTransactionsHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/analytics/fees" }, AnalyticsFeesHandler);
        self.register(RoutePattern { method: "GET", path: "/v1/analytics/settlements" }, AnalyticsSettlementsHandler);
    }
}
```

---

## 8. Request Size & Timeout Limits

```rust
pub struct RequestLimits {
    pub max_json_body: usize,           // 1MB (standard REST requests)
    pub max_protobuf_body: usize,       // 1MB (standard protobuf requests)
    pub max_document_upload: usize,     // 10MB (document upload)
    pub max_payment_creation: usize,    // 100KB
    pub general_timeout: Duration,      // 30s
    pub checkout_timeout: Duration,     // 10s
    pub analytics_timeout: Duration,    // 60s (complex queries)
}
```

---

## 9. Input Validation

Two-level validation:

### Level 1: Schema/Format Validation (at Gateway)

- **REST JSON**: JSON schema validation against OpenAPI 3.1 spec. Reject malformed JSON before forwarding
- **Protobuf**: Protobuf decode validation (field types, required fields, enum values)
- **Common**: UUIDv7 format, ISO 4217 currency codes, ISO 8601 timestamps
- **Security**: SQL injection pattern check, XSS sanitization, JSON depth limit (max 20 levels)

### Level 2: Semantic Validation (at Domain Service)

- Business rule validation (e.g., "amount must be positive")
- State machine validation (e.g., "cannot capture a voided intent")
- Handled by the receiving domain service

---

## 10. Error Response Format

### REST JSON Error Response

```json
{
  "error": {
    "type": "invalid_request_error",
    "code": "INSUFFICIENT_REFUNDABLE_BALANCE",
    "message": "Refund amount (6000 AED) exceeds remaining refundable balance (5000 AED)",
    "request_id": "req_abc123",
    "details": {
      "payment_intent_id": "pi_def456",
      "captured_amount": "5000",
      "refunded_amount": "0",
      "requested_amount": "6000"
    }
  }
}
```

### Protobuf Error Response

```protobuf
message ErrorDetail {
  string code = 1;
  string message = 2;
  string request_id = 3;
  map<string, string> details = 4;
}
```

**Error Response Rules**:
- **API-ERR-001**: Business errors return HTTP 200 with error detail in body (like Stripe)
- **API-ERR-002**: Transport-level errors use appropriate HTTP status codes (401, 403, 404, 429, 500, 503)
- **API-ERR-003**: Error codes are stable, documented enums — never free-text strings
- **API-ERR-004**: Every error response includes `request_id` for correlation

---

## 11. Response Envelope / Pagination

### REST JSON List Response

```json
{
  "data": [
    { "id": "pi_001", "amount": 10000, "status": "authorized", ... },
    { "id": "pi_002", "amount": 5000, "status": "captured", ... }
  ],
  "has_more": true,
  "next_cursor": "cursor_abc123",
  "total_count": 42
}
```

### REST JSON Single Resource Response

Direct JSON object — no wrapping envelope:

```json
{
  "id": "pi_001",
  "amount": 10000,
  "currency": "AED",
  "status": "authorized",
  "created_at": "2026-07-22T10:30:00Z",
  "metadata": { "order_id": "ORD-12345" }
}
```

---

## 12. Security Headers (HDR-001)

```http
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Content-Security-Policy: default-src 'self'
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()
X-XSS-Protection: 0
```

---

## 13. API Consistency Conventions

| Convention | Rule |
|---|---|
| **Naming** | `snake_case` for fields, `kebab-case` for URLs |
| **Timestamps** | ISO 8601 with millisecond precision: `2026-07-22T10:30:00.123Z` |
| **IDs** | Prefixed UUIDs: `pi_`, `li_`, `sub_`, `evt_`, `req_` (inspired by Stripe) |
| **Amounts** | Always in minor units (cents, fils): `10000` = `100.00 AED` |
| **Currencies** | ISO 4217 uppercase: `AED`, `USD`, `EUR` |
| **Pagination** | Cursor-based: `cursor` + `limit` (max 100) |
| **Idempotency** | `Idempotency-Key` header on all POST/PATCH requests |
| **Rate Limits** | Return `X-RateLimit-Remaining` and `X-RateLimit-Reset` headers |

---

## 14. TDD Tests

```rust
#[tokio::test]
async fn test_rest_json_create_payment_intent() {
    let request = json!({
        "amount": 10000,
        "currency": "AED",
        "idempotency_key": "key-001"
    });
    let response = gateway.handle_rest_json(
        "POST",
        "/v1/payment-intents",
        request.to_string(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["status"], "created");
    assert!(body["id"].as_str().unwrap().starts_with("pi_"));
}

#[tokio::test]
async fn test_rest_json_get_payment_intent() {
    let response = gateway.handle_rest_json(
        "GET",
        "/v1/payment-intents/pi_abc123",
        String::new(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_rest_json_list_with_filters() {
    let response = gateway.handle_rest_json(
        "GET",
        "/v1/payment-intents?status=authorized&limit=10&currency=AED",
        String::new(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert!(body["data"].is_array());
    assert!(body["has_more"].is_boolean());
}

#[tokio::test]
async fn test_protobuf_request_also_accepted() {
    let response = gateway.handle_protobuf(
        "/proto/orchestration.v1.OrchestrationService/CreatePaymentIntent",
        request_protobuf_bytes,
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_rest_json_invalid_body_rejected() {
    let response = gateway.handle_rest_json(
        "POST",
        "/v1/payment-intents",
        "not json".into(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 400);
}

#[tokio::test]
async fn test_expired_jwt_rejected() {
    let response = gateway.handle_rest_json(
        "POST",
        "/v1/payment-intents",
        json_request,
        Some(expired_jwt),
    ).await;
    assert_eq!(response.status, 401);
}

#[tokio::test]
async fn test_rate_limit_enforced() {
    // Send exceeding requests in 60 seconds
    // Should get 429
}

#[tokio::test]
async fn test_idempotency_key_respected() {
    let request = json_request.clone();
    // First request succeeds
    let r1 = gateway.handle_rest_json("POST", "/v1/payment-intents", request.clone(), Some(valid_jwt)).await;
    // Second with same key returns same result (not error)
    let r2 = gateway.handle_rest_json("POST", "/v1/payment-intents", request, Some(valid_jwt)).await;
    assert_eq!(r1.status, 200);
    assert_eq!(r2.status, 200);
    assert_eq!(r1.body, r2.body); // idempotent replay
}

#[tokio::test]
async fn test_security_headers_present() {
    let response = gateway.handle_rest_json("GET", "/v1/payment-intents/pi_001", String::new(), Some(valid_jwt)).await;
    assert!(response.headers.contains_key("strict-transport-security"));
    assert!(response.headers.contains_key("x-content-type-options"));
    assert!(response.headers.contains_key("x-frame-options"));
}

#[tokio::test]
async fn test_id_format_validated() {
    // IDs must follow prefix convention: pi_, li_, sub_, etc.
    let response = gateway.handle_rest_json(
        "GET",
        "/v1/payment-intents/invalid-id",
        String::new(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 400);
}
```
