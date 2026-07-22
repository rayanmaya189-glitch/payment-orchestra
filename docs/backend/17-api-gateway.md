# 17 — api-gateway (Cross-Cutting)

Single ingress for all external traffic. **REST paths + protobuf bodies**. Internal gRPC for service-to-service.

---

## 1. API Design Philosophy

The platform exposes a unified external API with RESTful URL patterns and protobuf-encoded request/response bodies. Internal service-to-service communication uses native gRPC.

| Aspect | External API | Internal |
|---|---|---|
| **URL Style** | RESTful paths (`/v1/payment-intents/:id`) | gRPC service methods |
| **HTTP Methods** | `POST`, `PATCH`, `DELETE` only | gRPC (HTTP/2) |
| **Content-Type** | `application/protobuf` (binary) | `application/grpc` |
| **Schema** | Protobuf messages | Same protobuf messages |

### Why REST Paths + Protobuf Bodies

- **Developer familiarity**: RESTful paths are intuitive and consistent across the industry. `/v1/payment-intents/:id` is immediately understood without reading documentation.
- **Type safety at wire level**: Protobuf binary encoding enforces schema validation, field types, and required fields at the protocol level — no runtime JSON parsing errors.
- **SDK code generation**: Protobuf definitions generate typed SDKs for all major languages automatically.
- **Performance**: Binary protobuf is smaller and faster to serialize/deserialize than JSON.
- **Single true schema**: The `.proto` file is the single source of truth for both external API and internal gRPC — no duplication between a REST JSON spec and protobuf definitions.

---

## 2. Endpoint Conventions

### 2.1 URL Structure

```
POST   /v1/{resource}              # Create resource
POST   /v1/{resource}/search       # List/Search resources (filters in protobuf body)
POST   /v1/{resource}/:id/{action} # Perform action on resource
PATCH  /v1/{resource}/:id          # Update resource
DELETE /v1/{resource}/:id          # Delete/disable resource
```

### 2.2 HTTP Methods

| Method | Purpose | Body | Idempotent |
|---|---|---|---|
| `POST` | Create, List/Search, Actions | Required (protobuf) | Yes (via Idempotency-Key header) |
| `PATCH` | Partial update | Required (protobuf) | Yes (via Idempotency-Key header) |
| `DELETE` | Delete/Disable | Optional (protobuf with reason) | Yes |
| `GET` | **NOT SUPPORTED** | — | — |

**GET is deliberately not supported.** All read operations use `POST` with the search/filter criteria in the protobuf body. This ensures:
- Consistent request/response format (always protobuf)
- Filter criteria can be complex nested protobuf messages (not limited by URL query string length)
- No ambiguity between query parameters and body parameters
- All operations can use the same authentication, rate limiting, and audit middleware

### 2.3 Core Payment Endpoints

```
POST   /v1/payment-intents                  # CreatePaymentIntent
POST   /v1/payment-intents/search           # ListPaymentIntents (cursor+filter in body)
PATCH  /v1/payment-intents/:id              # UpdatePaymentIntent (metadata, descriptor)
DELETE /v1/payment-intents/:id              # VoidPaymentIntent (delete = void)

# Sub-actions on a payment intent (POST because they create side effects)
POST   /v1/payment-intents/:id/authorize    # AuthorizePaymentIntent
POST   /v1/payment-intents/:id/capture      # CapturePaymentIntent
POST   /v1/payment-intents/:id/refund       # RefundPaymentIntent
POST   /v1/payment-intents/:id/void         # VoidPaymentIntent (alternative to DELETE)

# NEW: 3DS actions
POST   /v1/payment-intents/:id/check-3ds    # Check3DSEnrollment
POST   /v1/payment-intents/:id/authenticate-3ds # Authenticate3DS
```

### 2.4 BYOK Merchant Link Endpoints

```
POST   /v1/connectors                       # List available connectors (filters in body)
POST   /v1/connectors/search                # Search connectors
POST   /v1/connectors/:id/schema            # Get credential schema for connector
POST   /v1/merchant-links                   # Create MerchantAcquirerLink
POST   /v1/merchant-links/search            # List MerchantAcquirerLinks (cursor+filter in body)
PATCH  /v1/merchant-links/:id               # Update MerchantAcquirerLink metadata
DELETE /v1/merchant-links/:id               # Disable MerchantAcquirerLink
POST   /v1/merchant-links/:id/test          # Test connection
POST   /v1/merchant-links/:id/rotate        # Rotate credentials
```

### 2.5 Routing Endpoints

```
POST   /v1/routing-policies                 # Create RoutingPolicy
POST   /v1/routing-policies/search          # List RoutingPolicies
PATCH  /v1/routing-policies/:id             # Update RoutingPolicy
DELETE /v1/routing-policies/:id             # Delete RoutingPolicy
POST   /v1/routing-policies/:id/activate    # Activate RoutingPolicy
```

### 2.6 Webhook Endpoints

```
POST   /v1/webhook-endpoints                # Create WebhookEndpoint
POST   /v1/webhook-endpoints/search         # List WebhookEndpoints
PATCH  /v1/webhook-endpoints/:id            # Update WebhookEndpoint
DELETE /v1/webhook-endpoints/:id            # Delete WebhookEndpoint
POST   /v1/webhook-endpoints/:id/test       # Send test webhook event
POST   /v1/webhook-endpoints/:id/deliveries/search # List delivery history
POST   /v1/webhook-endpoints/:id/retry/:delivery_id # Retry failed delivery
```

### 2.7 Analytics & Reporting Endpoints

```
POST   /v1/analytics/transactions/search    # Transaction analytics (date range in body)
POST   /v1/analytics/fees/search            # Fee analysis
POST   /v1/analytics/settlements/search     # Settlement reports
POST   /v1/analytics/chargebacks/search     # Chargeback analytics
POST   /v1/reports/generate                 # Generate report (async, returns report_id)
POST   /v1/reports/:id/download             # Download generated report
```

---

## 3. Request/Response Format

All requests and responses use `Content-Type: application/protobuf` with protobuf binary encoding.

### Request

```
POST /v1/payment-intents
Content-Type: application/protobuf
Authorization: Bearer sk_live_abc123def456
Idempotency-Key: unique-key-001

<CreatePaymentIntentRequest protobuf binary>
```

### Response (Success)

```
HTTP/1.1 200 OK
Content-Type: application/protobuf
Request-Id: req_abc123

<PaymentIntent protobuf binary>
```

### Response (List/Search)

```
HTTP/1.1 200 OK
Content-Type: application/protobuf
Request-Id: req_abc123

<SearchPaymentIntentsResponse protobuf binary>
```

**Protobuf response for search:**

```protobuf
message SearchPaymentIntentsResponse {
  repeated PaymentIntent data = 1;
  bool has_more = 2;
  string next_cursor = 3;     // AES-256-GCM encrypted cursor
  int64 total_count = 4;      // approximate total (capped at 10,000)
  int64 as_of_unix_ms = 5;    // freshness timestamp
}
```

### Response (Error)

```
HTTP/1.1 200 OK (business errors)
HTTP/1.1 401/403/404/429/500 (transport errors)
Content-Type: application/protobuf
Request-Id: req_abc123

<ErrorDetail protobuf binary>
```

---

## 4. Example Flow: Create and Authorize Payment

```
// Step 1: Create PaymentIntent
POST /v1/payment-intents
Content-Type: application/protobuf
Authorization: Bearer sk_live_abc123
Idempotency-Key: create-pi-001

<CreatePaymentIntentRequest: amount=10000, currency="AED">

→ Response:
<PaymentIntent: id=pi_abc123, status=created>

// Step 2: Authorize PaymentIntent
POST /v1/payment-intents/pi_abc123/authorize
Content-Type: application/protobuf
Authorization: Bearer sk_live_abc123

<AuthorizePaymentIntentRequest: payment_method_token="tok_xyz">

→ Response:
<PaymentIntent: id=pi_abc123, status=authorized, acquirer_reference="ref_001">

// Step 3: Capture PaymentIntent
POST /v1/payment-intents/pi_abc123/capture
Content-Type: application/protobuf
Authorization: Bearer sk_live_abc123
Idempotency-Key: capture-pi-001

<CapturePaymentIntentRequest: amount=10000>

→ Response:
<PaymentIntent: id=pi_abc123, status=captured, captured_amount=10000>
```

---

## 5. Responsibilities

- **GW-001**: Single ingress point for all external protobuf-over-HTTP traffic
- **GW-002**: TLS termination, JWT/API-key validation, actor context extraction
- **GW-003**: Per-endpoint rate limiting (Redis sliding window)
- **GW-004**: Request routing: REST path + protobuf body → internal gRPC
- **GW-005**: Webhook signature verification NOT here (per-connector, see connector-gateway)
- **GW-006**: Protobuf schema validation before forwarding to backend services
- **GW-007**: Request/response logging for audit trail
- **GW-008**: URL parameter extraction (path variables → gRPC metadata)
- **GW-009**: Input sanitization and security header enforcement
- **GW-010**: CORS enforcement with configurable allowed origins

---

## 6. Authentication & Rate Limiting

### Authentication Flow

```
Request → TLS termination → Extract auth header
  → If JWT: validate signature, expiry, aud → extract principal_id
  → If API key: hash with Argon2id → lookup → extract scopes
  → Decode protobuf body, validate field constraints
  → Set actor_context in gRPC metadata
  → Forward to backend service via native gRPC
```

### API Key Prefix Convention

```
sk_live_*   = Secret key (live, full access)
sk_test_*   = Secret key (sandbox, full access) 
```

### Rate Limiting

```rust
pub struct RateLimitConfig {
    pub route: String,              // e.g., "POST /v1/payment-intents"
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
| `POST /v1/payment-intents/search` | 500 | 60s | ApiKey |
| `PATCH /v1/payment-intents/:id` | 500 | 60s | ApiKey |
| `POST /v1/merchant-links` | 20 | 60s | ApiKey |
| `POST /v1/routing-policies` | 10 | 60s | ApiKey |
| `POST /v1/webhook-endpoints` | 10 | 60s | ApiKey |
| `POST /v1/authenticate` | 10 | 60s | IP |
| `POST /v1/analytics/transactions/search` | 100 | 60s | ApiKey |
| `POST /v1/reports/generate` | 10 | 60s | ApiKey |

---

## 7. CORS Policy (CORS-001)

```rust
pub fn cors_middleware(allowed_origins: &[String]) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowedOrigins::list(allowed_origins))
        .allow_methods([POST, PATCH, DELETE, OPTIONS])   // No GET
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

## 8. URL → Internal Translation

The api-gateway translates REST path + protobuf body to internal gRPC calls:

```rust
pub struct RouteTranslator {
    routes: HashMap<String, TranslationRule>,
}

struct TranslationRule {
    http_method: String,       // POST, PATCH, DELETE
    url_pattern: String,       // e.g., "/v1/payment-intents/:id/capture"
    grpc_service: String,      // e.g., "orchestration.v1.OrchestrationService"
    grpc_method: String,       // e.g., "CapturePaymentIntent"
    path_params: Vec<String>,  // e.g., ["id"]
}

impl RouteTranslator {
    pub fn translate(&self, method: &str, path: &str, body: &[u8]) 
        -> Result<GrpcRequest, GatewayError> 
    {
        // 1. Match URL pattern (with path variable extraction)
        let rule = self.match_route(method, path)?;
        
        // 2. Extract path variables (e.g., :id → pi_abc123)
        let path_params = self.extract_path_variables(&rule.url_pattern, path)?;
        
        // 3. Validate protobuf body
        let validated_body = self.validate_protobuf(&rule, body)?;
        
        // 4. Build gRPC request with path params as metadata
        Ok(GrpcRequest {
            service: rule.grpc_service,
            method: rule.grpc_method,
            body: validated_body,
            metadata: path_params,
        })
    }
}
```

---

## 9. Request Size & Timeout Limits

```rust
pub struct RequestLimits {
    pub max_protobuf_body: usize,       // 1MB (standard requests)
    pub max_document_upload: usize,     // 10MB (document upload via streaming)
    pub max_payment_creation: usize,    // 100KB
    pub general_timeout: Duration,      // 30s
    pub checkout_timeout: Duration,     // 10s
    pub analytics_timeout: Duration,    // 60s (complex queries)
}
```

---

## 10. Input Validation

Two-level validation:

### Level 1: Protobuf Schema Validation (at Gateway)

- Protobuf decode validates field types, required fields, enum values
- UUIDv7 format, ISO 4217 currency codes, ISO 8601 timestamps
- Field-level range constraints (min, max, length)

### Level 2: Semantic Validation (at Domain Service)

- Business rule validation (e.g., "amount must be positive")
- State machine validation (e.g., "cannot capture a voided intent")
- Handled by the receiving domain service

---

## 11. Error Response Format

All errors return a protobuf-encoded `ErrorDetail`:

```protobuf
message ErrorDetail {
  string code = 1;              // "INSUFFICIENT_REFUNDABLE_BALANCE"
  string message = 2;           // Human-readable description
  string request_id = 3;        // Correlation ID: req_abc123
  map<string, string> details = 4; // Additional context: payment_intent_id, amount, etc.
}
```

**Error Response Rules**:
- **ERR-001**: Business errors (invalid state, validation) → HTTP 200 with `ErrorDetail` in body
- **ERR-002**: Transport errors (auth, rate limit, not found) → appropriate HTTP status (401, 403, 404, 429, 500, 503) with `ErrorDetail`
- **ERR-003**: Error codes are stable, documented enums — never free-text
- **ERR-004**: Every response includes `Request-Id` header for correlation

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
| **HTTP Methods** | `POST` (create/search/action), `PATCH` (update), `DELETE` (remove) |
| **URL Style** | `kebab-case`, plural resources: `/v1/payment-intents` |
| **Content-Type** | Always `application/protobuf` — no JSON, no form data |
| **ID Format** | Prefixed: `pi_`, `li_`, `sub_`, `evt_`, `req_` |
| **Amounts** | Always in minor units: `10000` = `100.00 AED` |
| **Currencies** | ISO 4217 uppercase: `AED`, `USD`, `EUR` |
| **Timestamps** | ISO 8601 milliseconds: `2026-07-22T10:30:00.123Z` |
| **Pagination** | Cursor-based in protobuf body: `cursor` + `limit` (max 100) |
| **Idempotency** | `Idempotency-Key` header on all `POST`/`PATCH` requests |
| **Rate Limits** | Return `X-RateLimit-Remaining` and `X-RateLimit-Reset` headers |
| **Correlation** | Return `Request-Id` header on all responses |

---

## 14. API Versioning

- **VER-001**: API versioned by URL prefix: `/v1/`, `/v2/`
- **VER-002**: Backward-compatible changes (new fields, new endpoints) within the same version
- **VER-003**: Breaking changes increment the version number and create a new URL prefix
- **VER-004**: During deprecation, old version receives `Sunset` header (RFC 8594) with removal date
- **VER-005**: Deprecation window: minimum 12 months for breaking changes

---

## 15. Protobuf Schema Evolution

- **EVOLVE-001**: New fields are additive (backward-compatible within a major version)
- **EVOLVE-002**: Breaking changes increment package version: `payment_intent.v2`
- **EVOLVE-003**: Deprecated fields marked with `deprecated = true` option, never removed
- **EVOLVE-004**: All `.proto` files live in the shared protobuf workspace crate

---

## 16. TDD Tests

```rust
#[tokio::test]
async fn test_create_payment_intent() {
    let body = CreatePaymentIntentRequest {
        amount: Some(Money { amount_minor_units: 10000, currency_code: "AED".into() }),
        idempotency_key: "key-001".into(),
        ..Default::default()
    };
    let response = gateway.handle_protobuf(
        "POST",
        "/v1/payment-intents",
        body.encode_to_vec(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
    let payment_intent: PaymentIntent = Message::decode(response.body.as_slice()).unwrap();
    assert_eq!(payment_intent.status, "created");
    assert!(payment_intent.id.starts_with("pi_"));
}

#[tokio::test]
async fn test_search_payment_intents() {
    let body = SearchPaymentIntentsRequest {
        status: Some("authorized".into()),
        currency: Some("AED".into()),
        limit: 10,
        ..Default::default()
    };
    let response = gateway.handle_protobuf(
        "POST",
        "/v1/payment-intents/search",
        body.encode_to_vec(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
    let result: SearchPaymentIntentsResponse = Message::decode(response.body.as_slice()).unwrap();
    assert!(result.has_more == true || result.data.len() > 0);
}

#[tokio::test]
async fn test_capture_payment_intent() {
    let body = CapturePaymentIntentRequest {
        amount: None, // full capture
        ..Default::default()
    };
    let response = gateway.handle_protobuf(
        "POST",
        "/v1/payment-intents/pi_abc123/capture",
        body.encode_to_vec(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_patch_update_payment_intent() {
    let body = UpdatePaymentIntentRequest {
        metadata: Some(HashMap::from([("order_id".into(), "ORD-123".into())])),
        ..Default::default()
    };
    let response = gateway.handle_protobuf(
        "PATCH",
        "/v1/payment-intents/pi_abc123",
        body.encode_to_vec(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_delete_void() {
    let response = gateway.handle_protobuf(
        "DELETE",
        "/v1/payment-intents/pi_abc123",
        vec![], // no body needed
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_get_not_supported() {
    let response = gateway.handle_raw(
        "GET",
        "/v1/payment-intents/pi_abc123",
        vec![],
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 405); // Method Not Allowed
}

#[tokio::test]
async fn test_malformed_protobuf_rejected() {
    let response = gateway.handle_protobuf(
        "POST",
        "/v1/payment-intents",
        b"not protobuf".to_vec(),
        Some(valid_jwt),
    ).await;
    assert_eq!(response.status, 400);
}

#[tokio::test]
async fn test_expired_jwt_rejected() {
    let body = CreatePaymentIntentRequest { ..Default::default() };
    let response = gateway.handle_protobuf(
        "POST",
        "/v1/payment-intents",
        body.encode_to_vec(),
        Some(expired_jwt),
    ).await;
    assert_eq!(response.status, 401);
}

#[tokio::test]
async fn test_idempotency_key_respected() {
    let body = CreatePaymentIntentRequest {
        idempotency_key: "idem-001".into(),
        ..Default::default()
    };
    let body_bytes = body.encode_to_vec();
    // First request succeeds
    let r1 = gateway.handle_protobuf("POST", "/v1/payment-intents", body_bytes.clone(), Some(valid_jwt)).await;
    // Second with same key returns same result
    let r2 = gateway.handle_protobuf("POST", "/v1/payment-intents", body_bytes, Some(valid_jwt)).await;
    assert_eq!(r1.status, 200);
    assert_eq!(r2.status, 200);
    assert_eq!(r1.body, r2.body);
}

#[tokio::test]
async fn test_rate_limit_enforced() {
    // Send exceeding requests in 60 seconds → 429
}

#[tokio::test]
async fn test_security_headers_present() {
    let response = gateway.handle_protobuf("POST", "/v1/payment-intents/search", vec![], Some(valid_jwt)).await;
    assert!(response.headers.contains_key("strict-transport-security"));
    assert!(response.headers.contains_key("x-content-type-options"));
    assert!(response.headers.contains_key("x-frame-options"));
}
```
