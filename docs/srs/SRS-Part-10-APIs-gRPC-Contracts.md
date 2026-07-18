# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

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
| Scope | External REST API conventions, internal gRPC contract conventions, webhook contract (outbound to merchants), event schema versioning rules, SDK strategy. |

---

## 1. API Design Conventions (REST — External Surface)

### 1.1 Versioning

- **API-001**: URL path versioning: `/v1/...`. A new major version is introduced only for breaking changes; additive fields/endpoints ship within the existing version.
- **API-002**: Minimum deprecation window for any `/v1` breaking change requiring a `/v2`: a defined period (e.g., 12 months) during which both versions are supported, publicly announced in advance — the exact window is a commercial/support-policy decision to be finalized in Part 12, but the *existence* of a guaranteed minimum window is a firm requirement, not optional.

### 1.2 Resource Conventions

- **API-003**: Resource-oriented REST (`/v1/payment-intents/{id}`, `/v1/invoices/{id}`), verbs only for genuine actions with no natural resource noun (`/v1/payment-intents/{id}/capture`, `/v1/payment-intents/{id}/void`).
- **API-004**: All mutating requests (`POST`, `PATCH`) that represent a "try to do this financial thing" accept an `Idempotency-Key` header, mapped directly to Part 5 §4.1's caller-facing idempotency mechanism — this is not optional per endpoint; it's a platform-wide contract rule for any endpoint that creates or mutates a money-movement-relevant resource.
- **API-005**: Standard error envelope:

```json
{
  "error": {
    "code": "INSUFFICIENT_REFUNDABLE_BALANCE",
    "message": "Requested refund amount exceeds the remaining refundable balance.",
    "request_id": "01HZ...",
    "details": { "remaining_refundable_minor_units": 50000, "currency": "AED" }
  }
}
```
`code` is a stable, documented machine-readable enum (never a free-text string merchants are expected to parse); `message` is human-readable and may change wording without being a breaking change; `request_id` ties back to the correlation ID (Part 3 §4 envelope, Part 9 §1.1) for support/debugging.

- **API-006**: Pagination via cursor (`?cursor=...&limit=...`), never raw offset-based pagination, since offset pagination degrades badly and produces inconsistent results under concurrent writes to the underlying eventually-consistent projections (Part 9 §6 XSTORE-001).
- **API-007**: Every list/read endpoint that surfaces data derived from an eventually-consistent projection (Part 3 §7) includes an `as_of` timestamp in the response envelope, directly implementing Part 9 XSTORE-001's freshness-disclosure requirement.

### 1.3 Authentication Headers

- **API-008**: `Authorization: Bearer <JWT>` for dashboard-session-derived calls; `Authorization: Basic` (API key : secret, base64) or a dedicated `X-Api-Key`/`X-Api-Secret` header pair for server-to-server calls (final header scheme choice to be confirmed against SDK ergonomics in Part 11, but the two distinct authentication paths — human-session vs. machine-key — are fixed).

---

## 2. gRPC Design Conventions (Internal Surface)

### 2.1 Representative Proto — Payment Orchestration Service

```protobuf
syntax = "proto3";
package orchestration.v1;

message Money {
  int64 amount_minor_units = 1;
  string currency_code = 2; // ISO 4217
}

message CreatePaymentIntentRequest {
  Money amount = 1;
  string idempotency_key = 2;
}

message CreatePaymentIntentResponse {
  string payment_intent_id = 1;
  string status = 2; // Created
}

message AuthorizePaymentIntentRequest {
  string payment_intent_id = 1;
  string payment_method_token = 2;
}

message AuthorizePaymentIntentResponse {
  string status = 1;              // Authorized | Failed | FailedAllRoutes
  repeated RoutingAttemptResult attempts = 2;
}

message RoutingAttemptResult {
  string acquirer_connector_id = 1;
  bool approved = 2;
  string normalized_decline_reason = 3; // empty if approved
  uint32 latency_ms = 4;
}

service OrchestrationService {
  rpc CreatePaymentIntent(CreatePaymentIntentRequest) returns (CreatePaymentIntentResponse);
  rpc AuthorizePaymentIntent(AuthorizePaymentIntentRequest) returns (AuthorizePaymentIntentResponse);
  rpc CapturePaymentIntent(CapturePaymentIntentRequest) returns (CapturePaymentIntentResponse);
  rpc VoidPaymentIntent(VoidPaymentIntentRequest) returns (VoidPaymentIntentResponse);
  rpc RefundPaymentIntent(RefundPaymentIntentRequest) returns (RefundPaymentIntentResponse);
  rpc GetPaymentIntent(GetPaymentIntentRequest) returns (PaymentIntentView);
}
```

- **GRPC-001**: Actor context is carried in call metadata by API Gateway — the service layer validates this context on every request.
- **GRPC-002**: All money fields use the shared `Money` message (integer minor units, Part 3 PRIN-04) across every service's proto definitions — this is a genuinely shared library type (a small shared proto package, `common.v1`), one of the few deliberate exceptions to "services own their own contracts," since inconsistent money representation across service boundaries would be a correctness hazard.

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
  int64 occurred_at_unix_ms = 6;
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

### 3.2 Representative Payload

```json
{
  "event_id": "01HZQ...",
  "event_type": "payment.authorized",
  "occurred_at": "2026-07-16T10:15:00Z",
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
- **SDK-002**: SDKs wrap idempotency-key generation (API-004) by default (auto-generating a UUID per logical operation unless the caller supplies their own), so merchant developers get safe-by-default retry behavior without needing to understand the full idempotency mechanism up front — this directly serves Persona "Rashid" (Part 1 §9) who needs to integrate quickly without becoming a payments-idempotency expert on day one.
- **SDK-003**: A client-side (browser/mobile) SDK for hosted-checkout/payment-link embedding (PROC-04) is a separate, thinner SDK — it never handles raw card data (tokenization happens via the acquirer/PSP's own client-side tokenization library, wrapped behind a consistent platform-provided interface) to preserve PCI scope minimization (Part 8 §4.3).

---

## 5. Rate Limiting Contract

- **RL-001**: Rate limits are disclosed via standard headers (`X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`) on every response, not just on 429s, so integrators can proactively pace requests.
- **RL-002**: AI Assistant endpoints (routed via `ai-gateway`, Part 4 §3) have a *separate* quota dimension from general API rate limits (Part 9 §2 `ai_quota:*` keys).

---

## 6. gRPC Service Versioning (Internal)

- **GRPC-VER-001**: Internal gRPC services use package-level versioning: `orchestration.v1`, `orchestration.v2`, etc. Breaking changes to internal gRPC contracts require a new package version.
- **GRPC-VER-002**: During a version transition, the old version remains available for at least one release cycle (N-1 compatibility) to allow dependent services to upgrade gracefully. The old version is marked as deprecated in the proto definition.
- **GRPC-VER-003**: Event schema versioning (Part 3 §4, Part 10 §2.2 GRPC-003) is independent of gRPC service versioning — event payloads evolve via `event_version` within the same service version.

---

## 7. Webhook Replay Protection

- **WEBHOOK-REPLAY-001**: Outbound webhook payloads include a `timestamp` field (UTC ISO 8601) and a nonce (`event_id`). Merchants should reject webhooks with timestamps older than a configurable window (default: 5 minutes) to prevent replay attacks.
- **WEBHOOK-REPLAY-002**: The HMAC-SHA256 signature (WEBHOOK-002) is computed over `timestamp + "." + event_id + "." + body` rather than body alone, binding the signature to a specific time window and event instance.
- **WEBHOOK-REPLAY-003**: Merchants are documented (SDK/docs) to maintain a set of recently-processed `event_id` values and reject duplicates within a configurable dedup window (default: 24 hours), complementing the platform's own at-least-once delivery semantics.

---

## 8. SDK Deprecation and Migration

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

## 10. Open Items Carried Forward

- **OQ-023**: Confirm final API-002 deprecation-window duration with Product/Legal.
- **OQ-024**: Confirm final SDK language priority order (§4 SDK-001) against actual pilot-merchant technology stack survey results.
- **OQ-025**: Confirm authentication header scheme for machine clients (§1.3 API-008).
- **OQ-048**: Finalize webhook replay window (§7 WEBHOOK-REPLAY-001, default 5 minutes) — too short causes legitimate delayed webhooks to be rejected; too long increases replay attack surface.
- **OQ-049**: Confirm whether `X-SDK-Version` header tracking (§8 SDK-DEP-003) is required for MVP or deferred to H2 SDK ecosystem maturity.

---

*End of Part 10. Proceed to Part 11: Testing, DevOps & Deployment.*
