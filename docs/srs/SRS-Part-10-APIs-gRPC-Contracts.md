# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 10 of 12:** APIs, gRPC Contracts & Event Schemas
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 10 of 12 — APIs & gRPC Contracts |
| Depends On | Part 3 (domain events/commands), Part 4 (service boundaries, gateway routing), Part 5–7 (orchestration/AI/connector internals whose public surface is formalized here), Part 9 (schemas the API surfaces reflect) |
| Feeds Into | Part 11 (contract testing as part of CI/CD, SDK generation pipeline) |
| Scope | External protobuf-over-HTTP API conventions, internal gRPC contract conventions, webhook contract (outbound to merchants), event schema versioning rules, SDK strategy. |

---

## 1. API Design Conventions — REST Paths + Protobuf Bodies

### 1.1 Core Principle

**PROTO-001**: All external API requests and responses use protobuf binary encoding. RESTful URL paths are used for resource identification. HTTP methods: POST, PATCH, DELETE only. No GET, no JSON, no form data.

**PROTO-002: URL patterns follow REST conventions: POST /v1/{resource} (create), PATCH /v1/{resource}/:id (partial update), DELETE /v1/{resource}/:id (remove). Resource actions use POST /v1/{resource}/:id/{action}.

**PROTO-003: List/search operations use POST /v1/{resource}/search with filter/cursor/limit in the protobuf body. Never GET with query parameters.

### 1.2 Why Protobuf-Only

- **Type safety**: Request/response schemas enforced at wire level, not by documentation
- **Code generation**: SDK generation from `.proto` files for all client languages
- **Consistency**: Same schema for external API, internal gRPC, and event payloads
- **Performance**: Binary serialization is smaller and faster than JSON
- **Evolution**: Additive field changes are backward-compatible by design

### 1.3 Versioning

- **API-001**: Package-based versioning: `orchestration.v1`, `orchestration.v2`. A new major version is introduced only for breaking changes; additive fields ship within the existing version.
- **API-002**: Minimum deprecation window for any breaking change requiring a new version: 12 months during which both versions are supported.

### 1.4 Request/Response Convention

```protobuf
// Every request includes idempotency key for mutating operations
message CreatePaymentIntentRequest {
  string idempotency_key = 1;
  Money amount = 2;
  string purpose = 3;          // 'payment' | 'card_verification'
  string source_type = 4;      // 'merchant_api' | 'invoice' | 'subscription' | 'payment_link'
  string source_id = 5;        // optional: invoice_id, subscription_id, etc.
  string payment_method_token_id = 6; // optional: for recurring payments
  map<string, string> metadata = 7;   // optional: merchant metadata
}
```

- **API-004**: All mutating requests accept an `idempotency_key` field in the protobuf body (not a header), mapped to Part 5 §4.1's caller-facing idempotency mechanism.
- **API-005**: Standard error response:

```protobuf
message ErrorDetail {
  string code = 1;              // machine-readable enum
  string message = 2;           // human-readable
  string request_id = 3;        // correlation ID
  map<string, string> details = 4;
}
```

- **API-006**: Pagination via cursor in request body (not query string):

```protobuf
message ListPaymentIntentsRequest {
  string cursor = 1;            // opaque cursor from previous response
  uint32 limit = 2;             // max 100, default 20
  string status_filter = 3;     // optional: filter by status
  int64 created_after_unix_ms = 4;  // optional: time range filter
  int64 created_before_unix_ms = 5; // optional: time range filter
}

message ListPaymentIntentsResponse {
  repeated PaymentIntentView items = 1;
  string next_cursor = 2;
  bool has_more = 3;
  int64 as_of_unix_ms = 4;      // freshness disclosure (XSTORE-001)
}
```

- **API-007**: Every list/read response includes `as_of_unix_ms` timestamp for freshness disclosure.

### 1.5 Authentication

- **API-008**: `Authorization: Bearer <JWT>` for dashboard-session-derived calls; `X-Api-Key` / `X-Api-Secret` header pair for server-to-server calls. Both carried in HTTP headers, validated by API Gateway before protobuf decoding.

---

## 2. gRPC Design Conventions (Internal Surface)

### 2.1 Service-to-Protocol Mapping

| Interaction | Protocol | Rationale |
|---|---|---|
| Any service → Any service (sync) | gRPC (Protobuf) | Type-safe, observable, traceable (Part 4 RULE-001) |
| Any service → Any service (async events) | NATS JetStream | Fan-out, decoupled, durable (Part 4 RULE-002) |
| External API → API Gateway | Protobuf-over-HTTP POST | Binary, type-safe, code-gen ready |

### 2.2 Schema Registry

- **SCHEMA-REG-001**: A centralized schema registry manages all `.proto` files used for gRPC contracts. The registry is a Git repository with CI validation that ensures:
  - All `.proto` files compile without errors
  - Breaking changes (field removal, type change) are detected automatically
  - Schema versions follow semantic versioning (GRPC-VER-001)

- **SCHEMA-REG-002**: Services generate their gRPC clients and servers from the shared schema registry, ensuring type safety across all internal communication.

### 2.3 Representative Proto — Payment Orchestration Service

```protobuf
syntax = "proto3";
package orchestration.v1;

import "common.v1/money.proto";

message CreatePaymentIntentRequest {
  string idempotency_key = 1;
  common.v1.Money amount = 2;
  string purpose = 3;          // 'payment' | 'card_verification'
  string source_type = 4;      // who initiated this payment
  string source_id = 5;        // optional: invoice_id, subscription_id, etc.
  string payment_method_token_id = 6; // optional: for recurring payments
  map<string, string> metadata = 7;
}

message CreatePaymentIntentResponse {
  string payment_intent_id = 1;
  string status = 2;
  int64 created_at_unix_ms = 3;
}

message AuthorizePaymentIntentRequest {
  string payment_intent_id = 1;
  string payment_method_token_id = 2;
}

message AuthorizePaymentIntentResponse {
  string status = 1;
  repeated RoutingAttemptResult attempts = 2;
  string three_ds_data_json = 3; // 3DS passthrough (opaque to platform)
  double risk_score = 4;
  string risk_level = 5;
}

message RoutingAttemptResult {
  string acquirer_connector_id = 1;
  bool approved = 2;
  string normalized_decline_reason = 3;
  uint32 latency_ms = 4;
  string gateway_profile_id = 5;
  int64 fee_minor_units = 6;
}

message CapturePaymentIntentRequest {
  string payment_intent_id = 1;
  int64 amount_minor_units = 2;  // 0 = full capture
  string currency_code = 3;
}

message CapturePaymentIntentResponse {
  string status = 1;
  int64 captured_amount_minor_units = 2;
  int64 remaining_authorized_minor_units = 3;
}

message VoidPaymentIntentRequest {
  string payment_intent_id = 1;
}

message VoidPaymentIntentResponse {
  string status = 1;
}

message RefundPaymentIntentRequest {
  string payment_intent_id = 1;
  int64 amount_minor_units = 2;
  string currency_code = 3;
  string reason = 4;
}

message RefundPaymentIntentResponse {
  string status = 1;
  int64 refunded_amount_minor_units = 2;
  int64 remaining_refundable_minor_units = 3;
}

message GetPaymentIntentRequest {
  string payment_intent_id = 1;
}

message PaymentIntentView {
  string payment_intent_id = 1;
  string status = 2;
  int64 requested_amount_minor_units = 3;
  int64 authorized_amount_minor_units = 4;
  int64 captured_amount_minor_units = 5;
  int64 refunded_amount_minor_units = 6;
  string currency_code = 7;
  string source_type = 8;
  string source_id = 9;
  double risk_score = 10;
  string risk_level = 11;
  string gateway_profile_id = 12;
  string expected_settlement_date = 13;
  int64 created_at_unix_ms = 14;
  int64 updated_at_unix_ms = 15;
}

message ListPaymentIntentsRequest {
  string cursor = 1;
  uint32 limit = 2;
  string status_filter = 3;
  int64 created_after_unix_ms = 4;
  int64 created_before_unix_ms = 5;
}

message ListPaymentIntentsResponse {
  repeated PaymentIntentView items = 1;
  string next_cursor = 2;
  bool has_more = 3;
  int64 as_of_unix_ms = 4;
}

service OrchestrationService {
  rpc CreatePaymentIntent(CreatePaymentIntentRequest) returns (CreatePaymentIntentResponse);
  rpc AuthorizePaymentIntent(AuthorizePaymentIntentRequest) returns (AuthorizePaymentIntentResponse);
  rpc CapturePaymentIntent(CapturePaymentIntentRequest) returns (CapturePaymentIntentResponse);
  rpc VoidPaymentIntent(VoidPaymentIntentRequest) returns (VoidPaymentIntentResponse);
  rpc RefundPaymentIntent(RefundPaymentIntentRequest) returns (RefundPaymentIntentResponse);
  rpc GetPaymentIntent(GetPaymentIntentRequest) returns (PaymentIntentView);
  rpc ListPaymentIntents(ListPaymentIntentsRequest) returns (ListPaymentIntentsResponse);
}
```

- **GRPC-001**: Actor context is carried in call metadata by API Gateway — the service layer validates this context on every request.
- **GRPC-002**: All money fields use the shared `Money` message (integer minor units, Part 3 PRIN-04) across every service's proto definitions.

### 2.2 Event Schema (Protobuf, Ties to Part 3 §4 / Part 9 §1.1)

```protobuf
syntax = "proto3";
package events.v1;

message EventEnvelope {
  string event_id = 1;
  string aggregate_type = 2;
  string aggregate_id = 3;
  string event_type = 4;
  uint32 event_version = 5;
  int64 occurred_at_unix_ms = 6;  // milliseconds since epoch, 3-digit millisecond precision
  string actor_type = 7;
  string actor_id = 8;
  string causation_id = 9;
  string correlation_id = 10;
  bytes payload = 11; // event-type-specific message, e.g., PaymentAuthorizedV1
}

message PaymentAuthorizedV1 {
  string payment_intent_id = 1;
  string acquirer_connector_id = 2;
  common.v1.Money amount = 3;
  string acquirer_reference = 4; // opaque, BC-04 ACL artifact (Part 3 §3.1)
}
```

- **GRPC-003 (Schema evolution rule)**: Within one `event_version`, only additive, optional fields may be added (protobuf's forward/backward-compatible field-addition semantics) — removing or repurposing a field number, or changing a field's semantic meaning, requires incrementing `event_version` and publishing to a new subject suffix (Part 4 §4.2, `...v2`), with consumers explicitly migrated rather than assumed to auto-handle the new shape.

---

## 3. Webhook Contract (Outbound to Merchants)

### 3.1 Design

- **WEBHOOK-001**: Merchants register one or more webhook endpoint URLs per event category (payment lifecycle, invoice lifecycle, subscription lifecycle, dispute lifecycle) via `iam`-gated dashboard/API configuration (SVC-06/08/10 respectively expose this, not a single monolithic webhook-config service, since each context's event volume/relevance to a given merchant integration differs).
- **WEBHOOK-002**: Every outbound webhook payload is signed (HMAC-SHA256 over the raw body using a per-merchant-endpoint secret) with the signature delivered in a header (`X-Signature`), mirroring the same signature-verification discipline the platform itself requires of inbound acquirer webhooks (Part 7 §1.2) — symmetry here is deliberate, since merchants integrating the platform face the same spoofing risk the platform faces from acquirers.
- **WEBHOOK-003**: At-least-once delivery with exponential backoff retry (a bounded number of attempts over a bounded window, e.g., up to 24 hours) and a dead-letter surface in the dashboard showing failed deliveries for manual replay — mirrors the internal NATS at-least-once philosophy (Part 4 §4.2) extended to the merchant-facing boundary.
- **WEBHOOK-004**: Webhook payload bodies carry a stable `event_id` (matching the internal domain event's `event_id` where directly derived) so merchant-side consumers can perform their own idempotent-processing dedup, exactly mirroring the discipline the platform demands of its own NATS consumers (Part 4 §4.2).

### 3.2 Webhook Delivery Tracking

- **WH-TRACK-001**: Every webhook delivery attempt is recorded in a `webhook_delivery_log` table (SeaORM entity):

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "webhook_delivery_log")]
pub struct WebhookDeliveryLogModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub delivery_id: Uuid,
    pub endpoint_id: Uuid,          // references webhook endpoint config
    pub event_id: Uuid,             // the event being delivered
    pub event_type: String,
    pub attempt_number: i32,
    pub status: String,             // 'pending' | 'success' | 'failed' | 'dead_letter'
    pub http_status_code: Option<i32>,
    pub error_message: Option<String>,
    pub request_timestamp: DateTimeWithTimeZone,
    pub response_timestamp: Option<DateTimeWithTimeZone>,
    pub next_retry_at: Option<DateTimeWithTimeZone>,
    pub total_retries: i32,
}
```

- **WH-TRACK-002**: Merchants can query their webhook delivery history via `POST /v1/webhook-deliveries/search` with filters for event type, status, and date range. The response includes delivery status, attempt count, and failure reasons.
- **WH-TRACK-003**: Failed deliveries in the dead-letter state can be manually replayed via `POST /v1/webhook-deliveries/{delivery_id}/replay` (Admin role, Maker/Checker pattern). Replay is idempotent — replaying an already-successful delivery is a no-op.
- **WH-TRACK-004**: Webhook delivery metrics are exported: delivery success rate, average delivery latency, retry count distribution, dead-letter depth. These metrics feed into the operator dashboard (UC-070).

### 3.2 Representative Payload

```json
{
  "event_id": "01HZQ...",
  "event_type": "payment.authorized",
  "occurred_at": "2026-07-16T10:15:00.123Z",
  "data": {
    "payment_intent_id": "01HZP...",
    "amount": { "amount_minor_units": 10000, "currency_code": "AED" },
    "status": "Authorized"
  }
}
```

---

## 4. SDK Strategy

- **SDK-001**: Server-side SDKs generated substantially from the OpenAPI spec (derived from the same source-of-truth REST contract as API-001–008) for at least three languages at GA-plus (per Part 1 GOAL-011) — candidates: Node.js/TypeScript, Python, PHP, given regional e-commerce platform prevalence (WooCommerce/Magento-adjacent PHP shops are common among UAE SMB merchants) — final language priority to be confirmed with Product against actual pilot-merchant tech stacks.
- **SDK-002**: SDKs wrap idempotency-key generation (API-004) by default (auto-generating a UUIDv7 per logical operation unless the caller supplies their own), so merchant developers get safe-by-default retry behavior without needing to understand the full idempotency mechanism up front.
- **SDK-003**: A client-side (browser/mobile) SDK for hosted-checkout/payment-link embedding (PROC-04) is a separate, thinner SDK — it never handles raw card data (tokenization happens via the acquirer/PSP's own client-side tokenization library, wrapped behind a consistent platform-provided interface) to preserve PCI scope minimization (Part 8 §4.3).

---

## 5. Rate Limiting Contract

- **RL-001**: Rate limits are disclosed via standard headers (`X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`) on every response, not just on 429s, so integrators can proactively pace requests.
- **RL-002**: AI Assistant endpoints (routed via `ai-gateway`, Part 4 §3) have a *separate* quota dimension from general API rate limits (Part 9 §2 `ai_quota:*` keys).
- **RL-003**: Per-endpoint rate limits: checkout endpoints (`/v1/payment-intents`) have higher limits than admin endpoints (`/v1/routing-policies`). Login endpoints have strict limits (10 per IP per minute).
- **RL-004**: Rate limit response includes `Retry-After` header when rate-limited (429 response), indicating when the client can retry.

### 5.2 Rate Limit 429 Response Format

When a request is rate-limited, the API returns HTTP 429 with the following body:

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Please retry after the specified time.",
    "request_id": "01HZ...",
    "details": {
      "retry_after_seconds": 30,
      "limit": 100,
      "remaining": 0,
      "reset_at": "2026-07-16T10:15:30Z",
      "window": "60s"
    }
  }
}
```

- **RL-RESP-001**: The `retry_after_seconds` field indicates the minimum time the client should wait before retrying. This value is also available via the `Retry-After` response header.
- **RL-RESP-002**: The `reset_at` field indicates when the rate limit window resets and the full quota becomes available again.
- **RL-RESP-003**: The `details` object is included only in 429 responses; successful responses include rate limit information only in headers (RL-001).

---

## 6. API Security

### 6.1 Request Size and Timeout Limits

- **APISEC-001**: Request size limits enforced at API Gateway:
  - Standard requests: maximum 1MB
  - Document upload endpoints: maximum 10MB
  - Payment creation: maximum 100KB
  - Limits enforced before body parsing to prevent memory exhaustion (OWASP A04).

- **APISEC-002**: Request timeouts:
  - API Gateway → domain service: 30 seconds (general), 10 seconds (checkout path)
  - Domain service → external acquirer: configurable per connector (default: 15 seconds authorize, 30 seconds settlement)
  - Timeout violations return explicit `REQUEST_TIMEOUT` error (Part 10 API-005 error format).

### 6.2 Input Validation

- **APISEC-003**: All API inputs validated against OpenAPI/gRPC schema at API Gateway before reaching domain services:
  - Type validation (string, integer, enum)
  - Length/range validation (min/max, string length)
  - Format validation (email, UUIDv7, ISO 4217, ISO 8601 with 3-digit millisecond precision: `YYYY-MM-DDTHH:MM:SS.mmmZ`)
  - Required field validation
  - Pattern validation (regex for structured fields like trade license numbers)

- **APISEC-004**: Domain-specific semantic validation happens in command handlers (Part 3 PRIN-01) — Gateway handles syntactic; domain handles semantic (e.g., "amount must be positive," "currency must be supported").

### 6.3 CORS Policy

- **APISEC-005**: CORS configuration:
  - `Access-Control-Allow-Origin`: Only explicitly whitelisted origins (operator-configured)
  - `Access-Control-Allow-Methods`: GET, POST, PUT, PATCH, DELETE, OPTIONS
  - `Access-Control-Allow-Headers`: Content-Type, Authorization, X-Api-Key, X-Idempotency-Key, X-Request-ID
  - `Access-Control-Allow-Credentials`: true (for cookie-based auth)
  - `Access-Control-Max-Age`: 86400 (24 hours preflight cache)
  - Default: no cross-origin requests allowed if no origins whitelisted.

### 6.4 Error Handling Security

- **APISEC-006**: API error responses never expose:
  - Stack traces or internal error details
  - Database query errors or connection strings
  - File paths or internal service names
  - Version information that could reveal attack surface
  - Internal IP addresses or hostnames

- **APISEC-007**: Error codes are stable, documented enums (API-005). Generic error messages returned to callers; detailed context logged server-side.

### 6.5 Response Security

- **APISEC-008**: API responses never include:
  - Internal implementation details (framework versions, database types)
  - Debug information in production (stack traces, variable dumps)
  - Sensitive data in error details (partial credentials, internal IDs)

### 6.6 Webhook Security (Outbound)

- **APISEC-009**: Outbound webhook security:
  - HMAC-SHA256 signature over `timestamp.event_id.body` (Part 10 §7 WEBHOOK-REPLAY-002)
  - Replay protection via timestamp window (5 minutes) and event_id dedup (24 hours)
  - URL validation: only HTTPS endpoints, resolved IPs checked against private ranges (Part 8 SSRF-002)
  - IP allowlisting documented for merchants who need to whitelist platform IPs

### 6.7 gRPC Security

- **APISEC-010**: Internal gRPC communication security:
  - mTLS enforced at API Gateway; internal calls use in-process identity propagation (Part 4 §7)
  - Actor context propagated in call metadata (GRPC-001)
  - Request size limits: 4MB default, configurable per service
  - Deadlines enforced on all gRPC calls (matching API-002 timeout specifications)

- **GRPC-VER-001**: Internal gRPC services use package-level versioning: `orchestration.v1`, `orchestration.v2`, etc. Breaking changes to internal gRPC contracts require a new package version.
- **GRPC-VER-002**: During a version transition, the old version remains available for at least one release cycle (N-1 compatibility) to allow dependent services to upgrade gracefully. The old version is marked as deprecated in the proto definition.
- **GRPC-VER-003**: Event schema versioning (Part 3 §4, Part 10 §2.2 GRPC-003) is independent of gRPC service versioning — event payloads evolve via `event_version` within the same service version.

---

## 7. Webhook Replay Protection

- **WEBHOOK-REPLAY-001**: Outbound webhook payloads include a `timestamp` field (UTC ISO 8601 with 3-digit millisecond precision: `YYYY-MM-DDTHH:MM:SS.mmmZ`) and a nonce (`event_id`). Merchants should reject webhooks with timestamps older than a configurable window (default: 5 minutes) to prevent replay attacks.
- **WEBHOOK-REPLAY-002**: The HMAC-SHA256 signature (WEBHOOK-002) is computed over `timestamp + "." + event_id + "." + body` rather than body alone, binding the signature to a specific time window and event instance.
- **WEBHOOK-REPLAY-003**: Merchants are documented (SDK/docs) to maintain a set of recently-processed `event_id` values and reject duplicates within a configurable dedup window (default: 24 hours), complementing the platform's own at-least-once delivery semantics.

---

## 8. SDK Deprecation and Migration

### 8.1 API Key Management — Maker/Checker

- **APIKEY-MKCK-001**: API key generation follows the Maker/Checker pattern:
  - **Maker**: Developer or Finance Operator requests a new API key via the dashboard or API
  - **Checker**: Admin approves the key request (different principal than the Maker, per MKCK-002)
  - Key is not active until Checker approves — pending keys are visible in the dashboard but cannot authenticate

- **APIKEY-MKCK-002**: API key rotation follows the same Maker/Checker pattern:
  - **Maker**: Developer or Finance Operator requests key rotation
  - **Checker**: Admin approves the rotation
  - Old key remains active for the grace period (24 hours) after new key is approved

- **APIKEY-MKCK-003**: API key revocation can be performed by Admin without Maker/Checker (emergency action), but all revocations are logged in `change_history` with the revocation reason.

- **APIKEY-MKCK-004**: API key scoping changes (Part 8 §11 AUTHZ-002) require Maker/Checker:
  - **Maker**: Developer or Finance Operator requests scope change
  - **Checker**: Admin approves the scope change
  - Scope changes are logged with before/after scope comparison

### 8.2 SDK Deprecation and Migration

- **SDK-DEP-001**: When a breaking API change is introduced (API-001/002), the SDK changelog includes:
  - The deprecated API method/field with a removal timeline
  - The replacement method/field
  - A migration guide (code examples showing before/after)
- **SDK-DEP-002**: Deprecated SDK methods emit compile-time warnings (via `#[deprecated]` attributes in Rust/TypeScript, `@deprecated` JSDoc in JavaScript) rather than runtime warnings, catching issues at build time.
- **SDK-DEP-003**: SDK version detection: the SDK sends an `X-SDK-Version` header with every request, allowing the API Gateway to track which SDK versions are still in active use and plan deprecation timelines based on real adoption data.

---

## 9. Traceability

| Requirement | Realized By |
|---|---|
| BIZ-010 (routing/connector configurability) | Indirectly — API surface for routing policy config (UC-011) exposed per API-003 conventions |
| Part 5 §4.1 (caller-facing idempotency) | §1.2 API-004 |
| Part 9 XSTORE-001 (freshness disclosure) | §1.2 API-007 |
| Part 7 §1.2 (webhook signature symmetry) | §3.1 WEBHOOK-002 |
| Part 3 §4 event envelope | §2.2 protobuf `EventEnvelope` |
| GOAL-011 (SDK ecosystem) | §4 |
| gRPC service versioning (internal) | §6 GRPC-VER-001 through GRPC-VER-003 |
| Webhook replay protection | §7 WEBHOOK-REPLAY-001 through WEBHOOK-REPLAY-003 |
| SDK deprecation and migration | §8 SDK-DEP-001 through SDK-DEP-003 |

---

## 10. Gap Analysis Additions — API Security & Integration

### 10.1 Cursor Pagination Security

**API-CURSOR-001**: Cursors are encrypted tokens (AES-GCM) containing: `last_record_id`, `sort_key`, `filter_hash` (hash of the query's filter parameters), `operator_id`, and `expiry`.

**API-CURSOR-002**: On decode, the server validates: (a) decryption succeeds, (b) `filter_hash` matches current request filters (prevents cursor reuse across different queries), (c) `operator_id` matches (defense in depth), (d) expiry hasn't passed (cursors valid for 1 hour).

**API-CURSOR-003**: Cursor encryption key is per-deployment and never exposed to clients. Cursors are opaque strings — no internal schema details leak.

### 10.2 Webhook Payload Schema Versioning

**WEBHOOK-VER-001**: Webhook payloads include a `schema_version` field (e.g., `"schema_version": "2026-07-18"`). New fields are additive and optional within a schema version.

**WEBHOOK-VER-002**: Breaking changes (field removal, type changes) increment the schema version. Merchants can specify their preferred schema version when registering webhook endpoints.

**WEBHOOK-VER-003**: During a migration window (12 months), both old and new schema versions are delivered. Schema version is tracked in `webhook_delivery_log` for debugging.

### 10.3 Webhook Delivery Backpressure & Throttling

**WEBHOOK-BP-001**: Per-endpoint timeout: 10 seconds for webhook delivery. If the merchant's endpoint doesn't respond within 10 seconds, the attempt is recorded as failed and retried per WEBHOOK-003.

**WEBHOOK-BP-002**: Per-endpoint concurrent delivery limit: maximum 5 in-flight webhooks per endpoint. If the limit is reached, new deliveries queue with lower priority.

**WEBHOOK-BP-003**: Adaptive throttling: if a merchant's endpoint has a >50% failure rate over the last 100 deliveries, reduce delivery frequency (maximum 1 delivery per 30 seconds) and alert the merchant.

**WEBHOOK-BP-004**: Webhook delivery queue is priority-based: payment lifecycle events (highest) > invoice events > notification events (lowest). High-priority events bypass throttling.

**WEBHOOK-BP-005**: Webhook delivery SLAs:
- p95 delivery latency: < 30 seconds for payment events
- Delivery success rate: > 99.9% for correctly-configured endpoints
- Dead-letter retention: 30 days
- Merchant notification when > 10% of webhooks fail within an hour

### 10.4 API Response Staleness Disclosure

**API-STALE-001**: Read endpoints serving data from eventually-consistent projections include:
- `as_of` timestamp (Part 10 API-007) — when the data was last updated
- `as_of_lag_seconds` — how far behind the projection is relative to the event store
- When lag exceeds the maximum acceptable threshold (defined per read-model, e.g., 5 minutes for reconciliation exceptions, 15 minutes for analytics), the API returns a `503 Stale Data` response with a `Retry-After` header for mutation-dependent reads

### 10.5 Webhook Inbound Security Enhancement

**WEBHOOK-IN-001**: Inbound webhook endpoints (from acquirers) are rate-limited per connector to prevent abuse: maximum 100 webhooks per minute per acquirer.

**WEBHOOK-IN-002**: Webhook payload size limited to 1MB (matching APISEC-001 document upload limits).

**WEBHOOK-IN-003**: Webhook timestamp validation: webhooks with timestamps older than the configurable replay window (default: 5 minutes per OQ-048) are rejected with HTTP 400.

---

## 11. Gap Analysis Additions — Round 2

### 11.1 Complete API Error Code Catalog

**API-ERRORS-001**: All API error codes organized by domain:

**Payment Errors:**
| Code | HTTP Status | Description |
|---|---|---|
| `PAYMENT_INTENT_NOT_FOUND` | 404 | PaymentIntent with given ID does not exist |
| `INVALID_STATE_TRANSITION` | 409 | Command not valid for PaymentIntent's current state |
| `PAYMENT_INTENT_FAILED` | 409 | PaymentIntent in terminal failed state |
| `PAYMENT_INTENT_VOIDED` | 409 | PaymentIntent has been voided |
| `AUTHORIZATION_EXPIRED` | 409 | Authorization validity window has passed |
| `PAYMENT_INTENT_ALREADY_CAPTURED` | 409 | PaymentIntent already fully captured |
| `PAYMENT_INTENT_FULLY_REFUNDED` | 409 | PaymentIntent fully refunded |
| `PAYMENT_INTENT_AUTHORIZING` | 409 | PaymentIntent currently authorizing |
| `PAYMENT_INTENT_CAPTURING` | 409 | PaymentIntent currently capturing |
| `PAYMENT_INTENT_NOT_AUTHORIZED` | 409 | PaymentIntent not yet authorized |
| `INSUFFICIENT_AUTHORIZED_AMOUNT` | 400 | Capture amount exceeds authorized amount |
| `INSUFFICIENT_REFUNDABLE_BALANCE` | 400 | Refund amount exceeds refundable balance |
| `ACQUIRER_LINK_DISABLED` | 409 | Original acquirer link is disabled |
| `ZERO_AMOUNT_NOT_REFUNDABLE` | 400 | Cannot refund zero-amount authorization |
| `DUPLICATE_IDEMPOTENCY_KEY` | 409 | Idempotency key used with different payload |
| `PARTIAL_CAPTURE_NOT_SUPPORTED` | 400 | Connector does not support partial capture |
| `MAX_PARTIAL_CAPTURES_EXCEEDED` | 400 | Exceeded connector's partial capture limit |
| `DUPLICATE_ORDER_INVOICE` | 409 | Invoice already exists for this order reference |

**Routing Errors:**
| Code | HTTP Status | Description |
|---|---|---|
| `NO_ELIGIBLE_ROUTE` | 422 | No acquirer matches routing policy for this transaction |
| `ROUTING_POLICY_INACTIVE` | 409 | No active routing policy configured |
| `ALL_ACQUIRERS_DECLINED` | 402 | All acquirers in routing chain declined |

**Auth Errors:**
| Code | HTTP Status | Description |
|---|---|---|
| `AUTHENTICATION_REQUIRED` | 401 | Valid authentication credentials required |
| `INSUFFICIENT_PERMISSIONS` | 403 | Principal lacks required permission |
| `API_KEY_EXPIRED` | 401 | API key has expired |
| `ACCOUNT_LOCKED` | 423 | Account locked due to failed attempts |
| `MFA_REQUIRED` | 403 | Step-up MFA required for this operation |
| `MAKER_CHECKER_PENDING` | 409 | Operation requires Maker/Checker approval |

**General Errors:**
| Code | HTTP Status | Description |
|---|---|---|
| `VALIDATION_ERROR` | 400 | Request body validation failed |
| `RATE_LIMITED` | 429 | Rate limit exceeded (see Retry-After header) |
| `REQUEST_TIMEOUT` | 504 | Request processing timed out |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |
| `STALE_DATA` | 503 | Read model data exceeds staleness threshold |

### 11.2 Complete Webhook Event Type Catalog

**WEBHOOK-EVENTS-001**: All merchant-subscribable webhook event types:

**Payment Lifecycle:**
| Event Type | Payload |
|---|---|
| `payment.created` | payment_intent_id, amount, currency, status |
| `payment.authorized` | payment_intent_id, amount, currency, acquirer, status |
| `payment.captured` | payment_intent_id, captured_amount, currency, status |
| `payment.failed` | payment_intent_id, failure_reason, decline_code, status |
| `payment.voided` | payment_intent_id, status |
| `payment.refunded` | payment_intent_id, refund_amount, currency, status |
| `payment.partially_refunded` | payment_intent_id, refund_amount, remaining_refundable, status |
| `payment.expired` | payment_intent_id, status |

**Invoice Lifecycle:**
| Event Type | Payload |
|---|---|
| `invoice.created` | invoice_id, amount, currency, due_date |
| `invoice.sent` | invoice_id, recipient |
| `invoice.paid` | invoice_id, amount_paid, payment_intent_id |
| `invoice.overdue` | invoice_id, days_overdue |
| `invoice.cancelled` | invoice_id |

**Subscription Lifecycle:**
| Event Type | Payload |
|---|---|
| `subscription.created` | subscription_id, plan_id, status |
| `subscription.renewed` | subscription_id, payment_intent_id, period_end |
| `subscription.renewal_failed` | subscription_id, failure_reason, retry_count |
| `subscription.cancelled` | subscription_id, reason |

**Dispute Lifecycle:**
| Event Type | Payload |
|---|---|
| `chargeback.received` | chargeback_id, payment_intent_id, reason_code |
| `chargeback.resolved` | chargeback_id, outcome (won/lost/accepted) |

### 11.3 API Changelog Mechanism

**API-CHANGELOG-001**: A versioned API changelog maintained in the public docs repository plus a `/v1/changelog` endpoint returning recent changes in JSON. Every API version change documented with: date, change type (additive/breaking), affected endpoints, migration guidance.

**API-CHANGELOG-002**: Changelog is linked from API docs, SDK release notes, and email notifications to registered developer contacts.

### 11.4 Webhook Inbound Security — Round 2

**WEBHOOK-IN-004**: Inbound acquirer webhook endpoints validate that the acquirer reference in the webhook payload resolves to the correct `MerchantAcquirerLink` and owning operator — prevents webhook cross-operator processing if webhook secrets are shared.

**WEBHOOK-DEDUP-001**: `connector-gateway` maintains a webhook deduplication cache (Redis, TTL 24h) keyed on `{connector_id}:{acquirer_webhook_id}`. On receipt, check cache: if present, return HTTP 200 (ack) without reprocessing.

### 11.5 Payment Link URL Security

**PLINK-SEC-003**: Payment link URLs use cryptographically random tokens with 128-bit minimum entropy (prevents enumeration). Token format: `plink_{random_32_bytes_base62}`.

---

## 12. Open Items Carried Forward

- **OQ-023**: Confirm final API-002 deprecation-window duration with Product/Legal.
- **OQ-024**: Confirm final SDK language priority order (§4 SDK-001) against actual pilot-merchant technology stack survey results.
- **OQ-025**: Confirm authentication header scheme for machine clients (§1.3 API-008).
- **OQ-048**: Finalize webhook replay window (§7 WEBHOOK-REPLAY-001, default 5 minutes) — too short causes legitimate delayed webhooks to be rejected; too long increases replay attack surface.
- **OQ-049**: Confirm whether `X-SDK-Version` header tracking (§8 SDK-DEP-003) is required for initial launch or deferred to Phase 2 SDK ecosystem maturity.

---

*End of Part 10. Proceed to Part 11: Testing, DevOps & Deployment.*
