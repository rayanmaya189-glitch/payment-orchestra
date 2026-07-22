# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 4 of 12:** Microservice Architecture
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 4 of 12 — Microservice Architecture |
| Depends On | Part 3 (Bounded Contexts map 1:1, with two deliberate exceptions noted in §1.2) |
| Feeds Into | Part 5 (Orchestration Engine internals), Part 6 (AI Gateway/Assistant internals), Part 7 (Connector Framework internals), Part 9 (per-service database ownership), Part 10 (gRPC/REST contracts), Part 11 (deployment, K8s topology) |
| Core Principle | One microservice owns exactly one bounded context's write model; no service reaches into another service's database. All cross-service communication is either synchronous gRPC (request/response, same-transaction-latency needs) or asynchronous NATS JetStream events (cross-context side effects). |

---

## 1. Service Catalog

### 1.1 Mapping Bounded Contexts → Microservices

| Service ID | Service Name | Bounded Context(s) | Language/Runtime | ORM | Primary Datastore |
|---|---|---|---|---|---|
| SVC-01 | `operator-service` | BC-01 Operator Management | Rust (SeaORM) | SeaORM | PostgreSQL |
| SVC-02 | `iam-service` | BC-02 Identity & Access | Rust (SeaORM) | SeaORM | PostgreSQL + Redis (session/token cache) |
| SVC-03 | `compliance-service` | BC-03 Merchant Compliance (KYB) | Rust (SeaORM) | SeaORM | PostgreSQL |
| SVC-04 | `connector-gateway` | BC-04 Gateway Connector Framework | Rust (SeaORM) | SeaORM | PostgreSQL (connector config, decline mappings) |
| SVC-05 | `orchestration-service` | BC-05 Payment Orchestration | Rust (SeaORM) | SeaORM | PostgreSQL (event store) + Redis (idempotency cache, hot routing config) |
| SVC-06 | `invoice-service` | BC-06 Invoice Service | Rust (SeaORM) | SeaORM | PostgreSQL (event store — revised from CRUD) |
| SVC-07 | `payment-link-service` | BC-07 Payment Link Service | Rust (SeaORM) | SeaORM | PostgreSQL |
| SVC-08 | `subscription-service` | BC-08 Subscription Billing | Rust (SeaORM) | SeaORM | PostgreSQL (event store) |
| SVC-09 | `reconciliation-service` | BC-09 Settlement & Reconciliation | Rust (SeaORM) | SeaORM | PostgreSQL (event store) |
| SVC-10 | `dispute-service` | BC-10 Dispute Management | Rust (SeaORM) | SeaORM | PostgreSQL |
| SVC-11 | `risk-service` | BC-11 Fraud & Risk Scoring | Rust (SeaORM) | SeaORM | PostgreSQL + Redis (hot scoring cache) |
| SVC-12 | `ai-assistant-service` | BC-12 AI Payment Assistant | Rust (SeaORM) | SeaORM | PostgreSQL (pgvector for embeddings) + Postgres (conversation metadata) |
| SVC-13 | `document-service` | BC-13 Document Management | Rust (SeaORM) | SeaORM | PostgreSQL (metadata) + S3-compatible storage |
| SVC-14 | `notification-service` | BC-14 Notification Service | Rust (SeaORM) | SeaORM | PostgreSQL + Redis (delivery dedup) |
| SVC-15 | `analytics-service` | BC-15 Analytics & Reporting | Rust | PostgreSQL (initially) → ClickHouse (H2) | PostgreSQL → ClickHouse |
| SVC-17 | `api-gateway` | Cross-cutting (not a bounded context) | Rust (Axum) | SeaORM | Redis (rate-limit counters only) |
| SVC-18 | `ai-gateway` | Cross-cutting (middleware within api-gateway) | Rust | — | Redis (rate limits), Postgres (guardrail audit log) |
| SVC-19 | `notification-service` | Cross-cutting (merged with SVC-14) | Rust (SeaORM) | SeaORM | PostgreSQL (subscriptions, delivery logs) |
| **SVC-21** | **`merchant-acquirer-link-service`** | **BYOK Core (NEW)** | **Rust (SeaORM)** | **SeaORM** | **PostgreSQL** |

**NEW SVC-21**: `merchant-acquirer-link-service` — BYOK (Bring Your Own Key) core service. Owns the `MerchantAcquirerLink` aggregate that connects a merchant to a specific payment gateway using the merchant's own credentials. This is the foundational BYOK entity. See `docs/backend/21-merchant-acquirer-link-service.md` for full specification.

**SVC-19 Merged**: `webhook-delivery-service` is merged into `notification-service` (SVC-14). Both are outbound delivery mechanisms with different destinations (email/SMS vs webhook). Merging them reduces operational complexity while keeping delivery auditing centralized.

### 1.2 Deliberate Deviations from Strict 1:1 Mapping

- **SVC-17 (API Gateway) and SVC-18 (AI Gateway)** are cross-cutting infrastructure services with no owning bounded context from Part 3 — they exist for routing, authentication enforcement, rate limiting, and (for AI Gateway specifically) prompt/output guardrail enforcement. They own no domain data beyond operational counters/logs.
- **SVC-06 (Invoice) and SVC-07 (Payment Link)** are kept as separate deployable services despite being closely related in the domain model (§3.5, Part 3), because they have different scaling profiles (payment links are read-heavy/public-facing hosted pages; invoices are write-heavy/back-office) and different security postures (payment link hosted pages are unauthenticated public endpoints; invoice management is authenticated). Splitting them lets SVC-07 scale and be hardened independently without over-provisioning SVC-06.

---

## 2. API Gateway (SVC-17)

### 2.1 Responsibilities

- **GW-001**: Single ingress point for all external REST/gRPC-Web traffic (merchant dashboard, merchant server-to-server API calls, SDKs).
- **GW-002**: TLS termination, request authentication (validates JWT/API-key issued by SVC-02 IAM), and actor-context extraction attached to every downstream request — no request reaches a domain service without a resolved actor context.
- **GW-003**: Per-endpoint rate limiting (Redis-backed sliding window), protecting downstream services from excessive request volume.
- **GW-004**: Request routing to the correct backend service based on path/version (`/v1/payments/*` → SVC-05, `/v1/invoices/*` → SVC-06, etc.), with API versioning support (Part 10 defines the versioning policy in full).
- **GW-005**: Webhook signature verification pass-through configuration is NOT done here — inbound acquirer webhooks land on SVC-04 (`connector-gateway`) directly via dedicated, acquirer-specific signed endpoints, since each acquirer has a different signature/verification scheme that belongs in the ACL, not the generic gateway.

### 2.2 What the API Gateway Deliberately Does NOT Do

- No business logic, no domain validation beyond authentication/authorization gating — keeping its full responsibility set prevents it from becoming a second, undocumented home for domain rules that should live in Part 3's aggregates.

### 2.3 Revised Responsibilities (with Dual API Support)

- **GW-001**: Single ingress point for all external REST JSON + Protobov traffic
- **GW-002**: TLS termination, JWT/API-key validation, actor context extraction
- **GW-003**: Per-endpoint rate limiting (Redis sliding window)
- **GW-004**: Content-type negotiation (JSON vs Protobuf) and route to correct handler
- **GW-005**: REST JSON → internal domain logic translation; Protobuf → internal gRPC translation
- **GW-006**: Input validation at gateway level (JSON schema / protobuf decode, format enforcement)
- **GW-007**: CORS enforcement, security headers, request correlation

### 2.4 New: Merchant-Acquirer Link (BYOK) Endpoints

The API Gateway now routes BYOK-related endpoints:

```
POST   /v1/connectors                    # List available connectors
POST   /v1/connectors/:id/schema         # Get credential schema for connector
POST   /v1/merchant-links                # Create MerchantAcquirerLink
POST   /v1/merchant-links/search                # List MerchantAcquirerLinks
POST   /v1/merchant-links/search/:id            # Get MerchantAcquirerLink
POST   /v1/merchant-links/:id/test       # Test connection
POST   /v1/merchant-links/:id/rotate     # Rotate credentials
DELETE /v1/merchant-links/:id            # Disable MerchantAcquirerLink
```

---

## 3. AI Gateway (SVC-18)

### 3.1 Why a Separate Gateway from the General API Gateway

The AI Gateway exists because AI-Assistant traffic has distinct requirements that would otherwise bloat SVC-17: prompt-injection guardrails, output-citation verification (BIZ-023), AI-usage quota enforcement, and routing to the correct model tier (Qwen3 32B for reasoning vs. Qwen3-VL 8B for document/vision tasks). Concentrating these concerns in SVC-18 keeps SVC-12 (`ai-assistant-service`) focused purely on RAG orchestration logic (detailed in Part 6).

### 3.2 Responsibilities

- **AIGW-001**: Route assistant requests to SVC-12; route document/vision-heavy requests specifically toward the Qwen3-VL 8B model pool vs. the Qwen3 32B reasoning pool (model selection policy, refined in Part 6).
- **AIGW-002**: Enforce AI usage quotas/tiers (commercial model support).
- **AIGW-003**: Apply input guardrails (basic prompt-injection pattern screening on user-supplied content that will be embedded in a RAG prompt, e.g., content pulled from uploaded documents) before it reaches the model — detailed guardrail design in Part 6.
- **AIGW-004**: Log every AI request/response pair (with citations) to the guardrail audit log (Postgres) for compliance review (BIZ-023, BIZ-040 adjacent).
- **AIGW-005**: Circuit-break to a graceful degradation response ("Assistant temporarily unavailable, here are the raw records") if the Ollama inference pool is unhealthy or over capacity, rather than queuing indefinitely and degrading the whole platform's perceived reliability.

---

## 4. Inter-Service Communication

### 4.1 Communication Style Decision Matrix

| Interaction | Style | Protocol | Rationale |
|---|---|---|---|
| External API → API Gateway | **RESTful JSON (primary)** or **Protobuf-over-HTTP (secondary)** | HTTP/1.1 or HTTP/2 + JSON or Protobuf binary | Dual API: REST JSON for merchant adoption (Stripe-like), Protobuf for performance-sensitive use cases |
| API Gateway → domain service (synchronous) | **gRPC** | HTTP/2 + Protobuf | Low latency, strongly-typed contracts (Part 10) |
| `orchestration-service` → `connector-gateway` (authorize/capture/refund) | **gRPC** (synchronous, in the checkout hot path) | HTTP/2 + Protobuf | Must return a result within the latency budget (BR-020-2) |
| `orchestration-service` → `risk-service` (pre-authorization risk score) | **gRPC** (synchronous, in the checkout hot path) | HTTP/2 + Protobuf | Risk score needed before routing decision; must be fast |
| `orchestration-service` publishing domain events | **NATS JetStream** (async, durable) | NATS protocol | Multiple downstream consumers; publisher must not block |
| `reconciliation-service` ingesting settlement files | **Mixed**: webhook via gRPC at connector-gateway → published to NATS; polling/SFTP via scheduled job → published to NATS | NATS protocol | Decouples ingestion cadence from downstream processing |
| `ai-assistant-service` reading context | **gRPC** (direct read-model queries) | HTTP/2 + Protobuf | BC-12 is a read-only Conformist; must never acquire write path |
| `notification-service` triggering | **NATS JetStream** subscription | NATS protocol | Naturally asynchronous, at-least-once |
| `compliance-service` → `document-service` | **gRPC** (synchronous) | HTTP/2 + Protobuf | Request-response; caller needs OCR result |
| `analytics-service` consuming events | **NATS JetStream** subscription | NATS protocol | Pure event consumer, never called synchronously |
| Cross-service queries | **gRPC** (query endpoints) | HTTP/2 + Protobuf | Type-safe, observable, traceable |

### 4.2 gRPC vs NATS Decision Rules

- **RULE-001**: External API traffic uses **Protobuf-over-HTTP POST** — binary protobuf request/response bodies, no REST conventions (PROTO-001).
- **RULE-002**: Use **gRPC** when the caller needs a synchronous response within the request's latency budget (checkout path, pre-authorization checks, document OCR requests).
- **RULE-003**: Use **NATS JetStream** when the interaction is naturally asynchronous, fan-out to multiple consumers, or doesn't require the caller to wait for completion (domain event publishing, notification dispatch, analytics ingestion).
- **RULE-004**: Never use NATS for the checkout hot path (Create/Authorize/Capture) — the latency of NATS publish-ack is unnecessary overhead when a direct gRPC call is more appropriate.
- **RULE-005**: Never use gRPC for fan-out event distribution — the publisher would need to know and manage all consumers, defeating the decoupling benefit of event-driven architecture.
- **RULE-006**: All service-to-service calls use gRPC with Protobuf — SeaORM services generate clients from `.proto` files for type-safe inter-service communication.
- **RULE-007: External API uses RESTful URL paths with protobuf-encoded request/response bodies. HTTP methods: POST (create/search/action), PATCH (update), DELETE (remove). NO GET, no JSON, no form data.
- **RULE-008: Internal service-to-service communication uses native gRPC with the same protobuf schemas as the external API.

### 4.3 NATS JetStream Subject Taxonomy (Versioned)

Following the naming convention introduced in Part 3 §4:

```
events.<context>.<aggregate>.<event_name>.v<version>

Examples:
events.orchestration.payment_intent.payment_authorized.v1
events.orchestration.payment_intent.payment_captured.v1
events.orchestration.routing_policy.routing_policy_activated.v1
events.reconciliation.settlement_batch.settlement_batch_ingested.v1
events.reconciliation.settlement_batch.settlement_record_matched.v1
events.dispute.chargeback_case.chargeback_received.v1
events.invoice.invoice.invoice_paid.v1
events.subscription.subscription.subscription_renewed.v1
events.notification.notification.notification_sent.v1
events.document.document.document_ocr_completed.v1
```

- **NATS-VER-001**: Every subject includes a version suffix (`v1`, `v2`, ...) that corresponds to the event's `event_version` field (Part 10 §2.2 GRPC-003). When an event schema changes in a backward-incompatible way, the version is incremented and a new subject is published to — consumers explicitly migrate to the new subject rather than auto-handling the new shape.
- **NATS-VER-002**: Consumers subscribe to a specific version (e.g., `events.orchestration.payment_intent.payment_authorized.v1`) — not a wildcard — to ensure they receive only the schema they expect. This prevents version mismatch bugs where a consumer receives an event it doesn't understand.
- **NATS-VER-003**: During a version transition (v1 → v2), the publisher emits to BOTH subjects for a defined deprecation window (default: 30 days). Consumers are migrated during this window. After the window, v1 publishing stops.
- **NATS-VER-004**: Subject naming uses snake_case for all segments: `events.<context_snake>.<aggregate_snake>.<event_name_snake>.v<N>`. This is consistent across Go and Rust publishers.

- **Streams**: One JetStream stream per bounded context (e.g., `ORCHESTRATION_EVENTS`, `RECONCILIATION_EVENTS`), partitioned by subject wildcard, retained per the policy resolved in OQ-008 (Part 3) — to be finalized numerically in Part 9 alongside storage sizing.
- **Consumer groups**: Each downstream service creates a durable consumer per stream it subscribes to, with explicit ack after successful projection/side-effect processing, and dead-letter handling (redeliver with backoff, then park in a `*_DLQ` subject after N failed attempts) surfaced to SVC-07... correction: surfaced to an operational alert channel monitored by ACT-07 (Support/Ops Engineer, Part 2).
- **Exactly-once processing semantics**: NATS JetStream provides at-least-once delivery; **exactly-once effect** is achieved at the consumer level via idempotent projections keyed on `event_id` (dedup table per consumer), not assumed from the transport layer.

### 4.4 gRPC Contract Ownership

Each domain service publishes its own `.proto` service definition (full contracts in Part 10); `connector-gateway` additionally defines the internal `AcquirerConnector` gRPC-equivalent trait contract (Rust trait, Part 7) that concrete acquirer adapters implement in-process (not a network call per adapter — adapters are compiled into `connector-gateway` as plugins/modules, not separate microservices per acquirer, to avoid N-times deployment overhead for what are essentially stateless protocol translators).

---

## 5. Service-by-Service Responsibility Detail

*(Deep internals for SVC-05, SVC-04, SVC-12 are deferred to their dedicated Parts 5, 7, 6 respectively; this section covers the remaining services not given a dedicated part.)*

### 5.1 SVC-01 `operator-service`
- Owns operator registration (UC-001), operator status lifecycle, operator member list (before fine-grained roles, which SVC-02 owns).
- Exposes: `CreateOperator`, `GetOperator`, `UpdateOperatorStatus` (internal-only, called by `compliance-service` on KYB approval), `ListOperatorMembers`.
- Publishes: `OperatorRegistered`, `OperatorVerified`, `OperatorSuspended`.

### 5.2 SVC-02 `iam-service`
- Owns authentication (issuing/validating JWTs), ABAC policy definitions, threshold-based approval rules (OQ-006 resolved in Part 8 §2.2 ABAC-001).
- Exposes: `Authenticate`, `IssueToken`, `ValidatePermission` (called synchronously by API Gateway on every request — must be extremely low latency, hence Redis-backed permission cache alongside Postgres source of truth).
- Publishes: `PrincipalCreated`, `RoleAssigned`, `PermissionDenied`.
- **NFR note (forward reference to Part 11)**: `ValidatePermission` sits on the critical path of every single API call platform-wide; its p99 latency budget is the tightest of any service in the system.

### 5.3 SVC-03 `compliance-service`
- Owns `KybCase` lifecycle (UC-002), integrates with external KYB partner via its dedicated ACL adapter, routes to internal review queue (ACT-06) on partner fallback.
- Exposes: `SubmitKybEvidence`, `GetKybStatus`, `ReviewKybCase` (internal, ACT-06 only).
- Publishes: `KybCaseSubmitted`, `KybCaseApproved`, `KybCaseRejected`.
- Calls: `document-service` (SVC-13) to store evidence files and trigger OCR extraction.

### 5.4 SVC-06 `invoice-service` / 5.5 SVC-07 `payment-link-service`
- Both call `orchestration-service` (SVC-05) synchronously to create the underlying `PaymentIntent` when the end customer completes checkout, then subscribe to EVT-03/EVT-04/EVT-06/EVT-07/EVT-09 to update their own `Invoice`/`PaymentLink` state asynchronously (avoiding a tight coupling where invoice status update blocks the payment confirmation response to the end customer).

### 5.6 SVC-08 `subscription-service`
- Runs its own scheduled renewal trigger (internal cron, not relying on an external scheduler service for MVP — see §6) that calls SVC-05 to create renewal `PaymentIntent`s using stored tokens.
- Owns dunning-schedule execution (UC-031 AF-031a).

### 5.7 SVC-09 `reconciliation-service`
- Subscribes to EVT-03/04/06/07/09/10 from `orchestration-service` to build its internal view of "what should eventually settle," and separately ingests settlement files/webhooks (§4.1) to match against that view.
- Exposes: `GetReconciliationExceptions`, `ResolveReconciliationException` (UC-041).

### 5.8 SVC-10 `dispute-service`
- Subscribes to chargeback webhooks via `connector-gateway` → NATS; exposes `SubmitRepresentment`, `GetChargebackCase`.

### 5.9 SVC-11 `risk-service`
- Initial release: synchronous rule-based scoring call from `orchestration-service` before authorization (low-latency requirement, Redis-cached rule set). H3: asynchronous ML scoring feeding back into routing decisions (GOAL-009 adjacent) — architecture must not preclude this evolution (extensibility NFR, Part 8).

### 5.10 SVC-13 `document-service`
- Owns MinIO-backed blob storage abstraction and metadata; triggers OCR pipeline (Qwen3-VL 8B, via `ai-gateway`/`ai-assistant-service`) asynchronously on upload, publishes `DocumentOcrCompleted` with extracted structured fields for the calling context (e.g., `compliance-service`) to consume.

### 5.11 SVC-14 `notification-service`
- Subscribes broadly across event streams per §4.1; owns templated email/SMS/push dispatch; is the only service permitted to call external notification providers (email/SMS gateways) — no other service sends customer-facing notifications directly, to keep delivery auditing (BIZ-040-adjacent) centralized.

### 5.12 SVC-15 `analytics-service`
- Pure event consumer across effectively all streams; writes append-only into ClickHouse; exposes read-only query endpoints for dashboards (UC-070) and report export (UC-071). Never receives direct write commands from users — all its data is derived.

### 5.13 Merged: Webhook Delivery (within SVC-14 `notification-service`)
- **Decision**: Webhook delivery functionality is merged into `notification-service` rather than as a separate SVC-19 service. Both email/SMS and webhook notifications are outbound delivery mechanisms with similar reliability requirements (retry, backoff, delivery logging). Merging reduces operational complexity from 18 to 17 deployable services.
- Retry policy: exponential backoff (1s → 3s → 9s → ... → 8h, 8 attempts max)
- Payload signing: HMAC-SHA256 with merchant-specific secret
- Delivery log retention: 30 days
- Webhook endpoint CRUD available via merchant dashboard

### 5.14 New: SVC-21 `merchant-acquirer-link-service` (BYOK Core)
- **NEW SERVICE**: See full specification at `docs/backend/21-merchant-acquirer-link-service.md`
- Owns the `MerchantAcquirerLink` aggregate — the fundamental BYOK entity
- Manages credential lifecycle: validation, encryption, rotation, expiry monitoring
- Provides connection health monitoring per merchant-gateway link
- Integrates with `connector-gateway` for credential validation and onboarding schemas

---

## 6. Scheduling & Background Jobs

- **JOB-001**: Subscription renewal triggers (SVC-08, internal cron per billing cycle).
- **JOB-002**: Dunning retry execution (SVC-08).
- **JOB-003**: Settlement file polling for acquirers that don't support webhook push (SVC-09, via `connector-gateway` polling adapters).
- **JOB-004**: Invoice overdue transition + reminder trigger (SVC-06).
- **JOB-005**: AI Assistant retrieval-index refresh/compaction (SVC-12, periodic re-embedding of updated documents — detailed in Part 6).
- **JOB-006**: Reconciliation exception aging alerts (SVC-09 → SVC-14).
- **JOB-009**: Data retention enforcement — scheduled archival/purge of data exceeding configured retention periods (Part 8 AUD-001, Part 1 BIZ-051).
- **JOB-010**: Outbox relay health monitoring — checks relay lag and alerts if relay falls behind (Part 3 §9.2 OUTBOX-001).

For initial release, scheduling is implemented as in-process cron-style schedulers within the owning service; if job volume/complexity grows past H1, a dedicated scheduling service is a candidate future extraction.

### 6.1 Leader Election

- **LEADER-001**: All scheduled jobs (JOB-001 through JOB-010) use leader election to prevent duplicate execution when multiple service replicas are running. The leader election mechanism is built on Redis (using SETNX-based distributed locks with TTL) — no external coordination service is required for MVP.
- **LEADER-002**: Each job type elects exactly one leader across all replicas of the owning service. The leader runs the job on its configured schedule. If the leader dies (detected via lock TTL expiry), another replica acquires leadership within one TTL cycle (default: 30 seconds for subscription renewals, 60 seconds for settlement polling).
- **LEADER-003**: Leadership is re-acquired on a rolling basis — the same replica doesn't need to stay leader permanently. This distributes job execution across replicas over time.

### 6.2 Graceful Shutdown

- **SHUTDOWN-001**: Every service implements the following graceful shutdown sequence on SIGTERM:
  1. **Stop accepting new requests** (deregister from service mesh/load balancer)
  2. **Complete in-flight requests** (wait up to a configurable drain timeout, default: 30 seconds)
  3. **Flush event store writes** (ensure all pending outbox entries are committed)
  4. **Release leader election locks** (so another replica can immediately acquire leadership)
  5. **Close database connections** (clean Postgres/Redis/ClickHouse disconnects)
  6. **Exit** (return SIGTERM exit code 0 for clean termination)

- **SHUTDOWN-002**: The drain timeout is configurable per service: checkout-critical services (`orchestration-service`, `connector-gateway`) use a shorter drain (15 seconds) to minimize user-perceived latency during deployment; non-critical services (`analytics-service`, `notification-service`) use a longer drain (60 seconds) to avoid losing in-flight event processing.

- **SHUTDOWN-003**: Kubernetes pod lifecycle hooks (`preStop` hook + `terminationGracePeriodSeconds`) are configured to match the service's drain timeout, ensuring K8s doesn't force-kill pods before they've completed graceful shutdown.

### 6.3 Health Check Endpoints

Every service exposes the following HTTP endpoints on a dedicated health port (port 8081, distinct from the main service port):

- **HEALTH-001**: `GET /healthz` (Liveness) — returns HTTP 200 if the process is alive and its event loop is running. Used by Kubernetes liveness probe to detect deadlocked processes.
- **HEALTH-002**: `GET /readyz` (Readiness) — returns HTTP 200 only when the service can accept traffic. Checks:
  - Database connectivity (Postgres ping)
  - Redis connectivity (PING)
  - Event store writable (for event-sourced services)
  - Required dependencies healthy (for services with sync dependencies on other services)
- **HEALTH-003**: `GET /startupz` (Startup) — returns HTTP 200 only after the service has completed initialization (model loading for AI services, schema migration check, cache warming). Used by Kubernetes startup probe with a generous timeout (120 seconds for AI services, 10 seconds for others).
- **HEALTH-004**: `GET /healthz/deep` (Deep Health) — returns detailed health status of all subsystems (database, Redis, NATS, external acquirer connectivity). Used by operations dashboards, not by Kubernetes probes.

- **HEALTH-005**: Health endpoints never expose sensitive information (no internal IPs, connection strings, or version details). They return only status codes and generic status strings.

## 7. Event Replay & Recovery

### 6.4 DLQ (Dead Letter Queue) Pattern

- **DLQ-001**: Every NATS consumer that fails to process an event after a configurable maximum retry count (default: 5 retries) moves the event to a dedicated DLQ subject (`<stream_name>.DLQ`). The DLQ event retains the original event payload plus failure metadata (error message, retry count, timestamps).
- **DLQ-002**: DLQ events are stored in a dedicated `dlq_events` table (SeaORM entity) per service:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "dlq_events")]
pub struct DlqEventModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub event_id: Uuid,
    pub stream_name: String,
    pub original_subject: String,
    pub payload: Vec<u8>,
    pub error_message: String,
    pub retry_count: i32,
    pub first_failed_at: DateTimeWithTimeZone,
    pub last_attempt_at: DateTimeWithTimeZone,
    pub status: String,           // 'pending' | 'resolved' | 'discarded'
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTimeWithTimeZone>,
}
```

- **DLQ-003**: DLQ depth is monitored as a first-class metric. When DLQ depth exceeds a configurable threshold (default: 10 events), an alert is raised to operations (ACT-07). When DLQ depth exceeds a critical threshold (default: 100 events), the service is flagged as degraded.
- **DLQ-004**: DLQ events can be manually replayed via a dashboard (Admin role, Maker/Checker pattern) after the root cause is fixed. Replay is idempotent — replaying an already-processed event is a no-op.
- **DLQ-005**: DLQ events are retained for 30 days, then automatically discarded. This retention must exceed the maximum time to diagnose and fix a consumer bug.

### 6.5 Poison Pill Handling

- **POISON-001**: A poison pill is an event that causes a consumer to crash or fail on every processing attempt. Detection: if an event reaches the maximum retry count (DLQ-001) without successful processing, it is quarantined in the DLQ.
- **POISON-002**: The consumer continues processing subsequent events after quarantining a poison pill — it does not block the entire consumer. This is achieved by processing events individually (not batch-committing) and catching per-event exceptions.
- **POISON-003**: When a poison pill is detected, the consumer logs a `PoisonPillDetected` event with the original event ID, error details, and affected aggregate. This event is surfaced to the operational alert channel (ACT-07).
- **POISON-004**: Poison pill resolution requires a Developer or Admin to investigate the root cause, fix the consumer code (if it's a consumer bug) or the event payload (if it's a publisher bug), and then manually replay the quarantined event from the DLQ.

### 6.6 Consumer Backpressure Management

- **BACKPRESSURE-001**: Every NATS consumer monitors its processing lag (how far behind the consumer is relative to the publisher). Lag is exposed as a metric (e.g., `nats_consumer_lag{consumer="analytics-service", stream="orchestration"}`).
- **BACKPRESSURE-002**: When lag exceeds a configurable warning threshold (default: 10,000 events), the consumer logs a warning and raises an alert. When lag exceeds a critical threshold (default: 100,000 events), the consumer enters backpressure mode: it pauses consuming new events until lag drops below the warning threshold, preventing memory exhaustion.
- **BACKPRESSURE-003**: In backpressure mode, the consumer does NOT acknowledge events it hasn't processed — NATS redelivers them when the consumer resumes. This ensures no events are lost during backpressure.

### 6.7 gRPC Deadline Propagation

- **DEADLINE-001**: Every gRPC call propagates the deadline from the caller to the callee. The API Gateway sets the initial deadline based on the endpoint's latency budget (Part 10 APISEC-002).
- **DEADLINE-002**: When a service makes a downstream gRPC call, it derives the downstream deadline from its own remaining deadline minus a processing buffer (default: 100ms). If the remaining deadline is less than the buffer, the call is rejected immediately with `DEADLINE_EXCEEDED`.
- **DEADLINE-003**: The checkout hot path (API Gateway → orchestration-service → connector-gateway → acquirer) has a total deadline of 10 seconds (Part 10 APISEC-002). Each service in the chain uses at most 80% of its remaining deadline for downstream calls, reserving 20% for its own processing.
- **DEADLINE-004**: When a deadline expires, the service returns a gRPC `DEADLINE_EXCEEDED` status with a structured error (Part 10 API-005 format). The error is logged with the full call chain (correlation_id) for debugging.

### 6.8 Event Schema Registry

- **SCHEMA-REG-001**: A centralized schema registry (Git repository) manages all event schemas (protobuf definitions). The registry is the single source of truth for event payload formats.
- **SCHEMA-REG-002**: Every event type has a canonical schema file in the registry (e.g., `events/orchestration/payment_intent/PaymentAuthorizedV1.proto`). The schema version matches the NATS subject version suffix (v1, v2, etc.).
- **SCHEMA-REG-003**: CI validation ensures all schema files compile without errors and that backward-compatibility rules (additive fields only within a version) are enforced automatically.
- **SCHEMA-REG-004**: Publishers generate event payloads from the registry schemas; consumers generate deserialization code from the same schemas. This ensures type safety across the entire event-driven architecture.

- **REPLAY-001**: Every event-sourced service supports replaying events for a specific aggregate from a given sequence number. This is used for:
  - Rebuilding read models after a projection bug fix
  - Debugging aggregate state during incident investigation
  - Verifying event-sourcing correctness in production

- **REPLAY-002**: A replay CLI tool (`event-replay-cli`) can be invoked per-service to:
  1. Read events from the event store for a specified aggregate ID and sequence range
  2. Rebuild the aggregate's current state by folding events
  3. Rebuild read-model projections from the event stream
  4. Output a diff between the current projected state and the rebuilt state

- **REPLAY-003**: For full read-model rebuilds (e.g., after a ClickHouse schema change), a batch replay job processes all events in chronological order across all aggregates, rebuilding the entire projection. This is a long-running operation with progress tracking and the ability to resume from the last processed event.

### 7.2 Event Store Corruption Recovery

- **RECOVERY-001**: If an event is found to be corrupted (protobuf decode failure, checksum mismatch), the service marks the corrupted event with a `CORRUPTED` flag and continues processing subsequent events. A corruption alert is raised to operations (ACT-07).
- **RECOVERY-002**: Corrupted events can be manually replaced by an authorized operator (Admin role, Maker/Checker pattern) with a corrected event payload, logged in `change_history` with before/after payloads.
- **RECOVERY-003**: Event store integrity is verified periodically (daily) by a background job that checks protobuf decodability of all events in the event store. Any corruption is detected within 24 hours.

---

## 7. Authentication & Authorization at the Service Layer

- **AUTH-001**: Every gRPC request (internal, service-to-service) carries an `actor_context` metadata field populated by API Gateway (§2) at ingress and propagated unchanged through every downstream hop — no service is permitted to "look up" an actor from a body field for authorization purposes; only the propagated, gateway-verified context is trusted.
- **AUTH-002**: Every database query in every service follows the application's access-control pattern (detailed in Part 8) — code review and automated lint rules (Part 11) enforce that no raw query bypasses authorization checks.

---

## 8. Deployment Topology (Preview)

*(Full Kubernetes/Docker deployment specification is in Part 11; this section previews the shape so Parts 5–10 can assume a consistent mental model.)*

- Each service in §1.1 is an independently deployable, independently scalable container. `orchestration-service` and `connector-gateway` are provisioned with the highest replica-count floor and tightest autoscaling responsiveness, since they sit on the checkout-latency-critical path.
- `ai-assistant-service` and its Ollama inference backend are deployed on GPU-backed node pools, separate from the general CPU-only service mesh, with the `ai-gateway` mediating so that a spike in AI usage cannot starve GPU resources needed for anything else (there is nothing else GPU-bound at launch, but this isolation is kept as a forward-looking discipline).
- All services are deployed behind a service mesh providing mTLS between services (Part 8, zero-trust internal networking) — this is what makes the "trust only the propagated gateway context" claim enforceable rather than aspirational: services physically cannot be reached except through authenticated mesh identities.

---

## 9. Traceability to Part 1 / Part 2 / Part 3

| Requirement/Context | Realized By |
|---|---|
| BIZ-021 (self-hosted AI) | §3, §8 GPU-isolated `ai-assistant-service` deployment |
| BIZ-023 (citable AI answers) | §3.2 AIGW-004 guardrail audit log |
| BR-020-2 (bounded failover latency) | §4.1 synchronous gRPC choice for orchestration↔connector-gateway |
| BC-12 read-only Conformist (Part 3 §1.3) | §4.1 "direct read-model queries... never the write-model database" |

---

## 10. Gap Analysis Additions — Infrastructure & Operational Patterns

### 10.1 Messaging & Cache Encryption (Part 8 §4 Extension)

The SRS specifies TLS 1.3 for external traffic (Part 8 ENC-001) and mTLS for service-to-service (AUTH-006) but does not specify encryption for NATS JetStream or Redis — the two most sensitive internal data paths after the database.

**NATS-ENC-001**: All NATS client-server connections use TLS 1.3 with mTLS (service mesh certificates). No plaintext NATS connections are permitted in any environment (dev/staging/prod).

**NATS-ENC-002**: NATS JetStream message stores are encrypted at rest using AES-256-GCM via JetStream's native encryption configuration. Encryption keys are managed through the platform KMS (Part 8 SEC-001).

**NATS-ENC-003**: NATS connection configuration must specify `tls_client_cert_file` and `tls_client_key_file` for every service, with `tls_ca_file` pointing to the service mesh CA. Connection fails if TLS negotiation fails — no plaintext fallback.

**REDIS-ENC-001**: All Redis client-server connections use TLS with mutual authentication (mTLS or AUTH with TLS).

**REDIS-ENC-002**: Redis persistence files (AOF, RDB) are encrypted at rest via volume-level encryption at minimum. Application-layer encryption for sensitive cache entries (session/permission data) as defense-in depth.

**REDIS-ENC-003**: Redis `requirepass` or ACL-based authentication is mandatory. Default `redis.conf` with no authentication is not permitted in any environment.

**Implementation Note**: These controls are additions to Part 8 §4 (Encryption) and Part 9 §2 (Redis) / Part 9 §4 (NATS). The Part 8 §4.1 "In Transit" section should be extended to include "NATS JetStream" and "Redis" in the TLS scope.

### 10.2 Feature Flag Management Pattern

**FF-001**: A Redis-backed feature flag store provides per-tenant progressive rollout and kill-switch capabilities.

**FF-002**: Feature flag evaluation happens at two levels:
- **API Gateway level**: For cross-cutting flags (e.g., new checkout flow, rate-limit configuration)
- **Service command-handler level**: For domain-specific flags (e.g., new routing algorithm, AI model version)

**FF-003**: Flag changes are domain events (`FeatureFlagChanged`) consumed by all services for eventual consistency. Kill-switch flags propagate via Redis pub/sub for sub-second effect.

**FF-004**: All flag state changes follow the Maker/Checker pattern (Part 3 MKCK-001) and are audit-logged.

**FF-005**: Flag configuration schema:

```rust
pub struct FeatureFlag {
    pub flag_key: String,           // e.g., "routing.dynamic_weighted"
    pub enabled: bool,
    pub targeting: FlagTargeting,   // percentage rollout, user segment, tenant list
    pub kill_switch: bool,          // if true, overrides all targeting — immediate effect
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

pub enum FlagTargeting {
    Global(bool),
    Percentage(f64),                // 0.0-1.0, deterministic hash of entity_id
    TenantList(Vec<Uuid>),
    Segment(String),                // e.g., "enterprise", "sandbox"
}
```

### 10.3 Structured Log Schema Standard

**LOG-SCHEMA-001**: Every service emits structured JSON logs conforming to this schema:

```json
{
  "timestamp": "2026-07-18T10:15:00.123Z",
  "level": "INFO|WARN|ERROR|DEBUG",
  "service": "orchestration-service",
  "correlation_id": "01HZ...",
  "causation_id": "01HZ...",
  "actor_id": "01HZ...",
  "actor_type": "user|api_key|system",
  "event_type": "PaymentAuthorized",
  "message": "Payment authorized on Acquirer B",
  "metadata": {
    "payment_intent_id": "01HZ...",
    "acquirer": "acquirer_b",
    "latency_ms": 1250
  }
}
```

**LOG-SCHEMA-002**: The schema is enforced via a shared Rust crate (`platform-logging`) that all services import. Log output that doesn't conform to the schema is rejected at the serialization layer (compile-time enforcement via typed fields, not runtime regex).

**LOG-SCHEMA-003**: Sensitive fields (credentials, PAN, tokens, API keys) are explicitly excluded from the schema. A log-scrubbing layer applied before serialization strips any sensitive data that accidentally appears in `metadata` values.

### 10.4 Unified Error Taxonomy

**ERR-TAX-001**: A shared `InternalErrorCode` enum in the `common.v1` proto package standardizes error classification across all services:

```protobuf
enum InternalErrorCode {
  INTERNAL_ERROR_CODE_UNSPECIFIED = 0;
  TRANSIENT_FAILURE = 1;        // retryable (UNAVAILABLE, DEADLINE_EXCEEDED)
  PERMANENT_FAILURE = 2;        // do not retry (INVALID_ARGUMENT, NOT_FOUND)
  DEGRADED_MODE = 3;            // continue with degraded behavior
  RATE_LIMITED = 4;             // too many requests
  AUTHORIZATION_DENIED = 5;     // permission check failed
  VALIDATION_ERROR = 6;         // input validation failed
  UNAVAILABLE = 7;              // service dependency unavailable
}
```

**ERR-TAX-002**: Every gRPC response includes this code. Callers use the code to decide retry/degrade/fail behavior — not gRPC status codes alone (which are too coarse-grained for payment-system error handling).

### 10.5 NATS JetStream Trace Context Propagation

**NATS-TRACE-001**: The `EventEnvelope` (Part 3 §4) is extended with an optional `trace_context` field:

```protobuf
message EventEnvelope {
  // ... existing fields ...
  string trace_context = 12;  // W3C Trace Context format: "00-{trace_id}-{span_id}-{flags}"
}
```

**NATS-TRACE-002**: The outbox relay extracts the current OpenTelemetry trace context when publishing to NATS and stores it in the envelope.

**NATS-TRACE-003**: NATS consumers extract the trace context and create a **linked span** (not a child span — the temporal gap is unbounded) connecting the consumer's processing to the original command that produced the event.

**NATS-TRACE-004**: This provides causation linkage from consumer back to producer, enabling end-to-end incident investigation across the event-driven boundary.

### 10.6 Concurrent Request Rate Limiting

**RL-CONC-001**: In addition to time-window rate limiting (Part 10 RL-001), the API Gateway enforces concurrent request limits per API key (or per tenant for unauthenticated endpoints).

**RL-CONC-002**: Redis-backed concurrent request counter: on request entry, `INCR`; on completion, `DECR`. If counter exceeds configured max concurrent (default: 100 per API key for checkout endpoints), return HTTP 429 with `Retry-After`.

**RL-CONC-003**: A TTL safety net (30 seconds) auto-decrements leaked counters from crashed connections, preventing permanent lockout.

### 10.7 Egress Network Policies

**NET-EGRESS-001**: In addition to ingress NetworkPolicies (HARD-002), egress restrictions are specified:
- `connector-gateway`: egress restricted to known acquirer IP ranges (maintained in a ConfigMap, updated per connector)
- `notification-service`: egress restricted to known email/SMS provider IPs
- All other services: egress restricted to internal service mesh only (no direct external egress)
- DNS egress restricted to platform DNS resolver

---

## 11. Open Items Carried Forward

- **# OQ-009 (Resolved)**: Risk-service latency on checkout path — resolved: risk scoring is called synchronously with a timeout budget (100ms). If timeout exceeded, proceed with default routing (risk score = 0.0). See `11-risk-service.md` for details.
- **# OQ-010 (Resolved)**: Invoice-service and payment-link-service share a single PostgreSQL instance (different schemas) at launch. Separation is a future scaling option, not a launch requirement.

### 11.1 New: 3D Secure Integration

3D Secure (3DS) is **mandatory** for card payments in the UAE (Central Bank regulations) and Europe (PSD2 SCA). The platform integrates 3DS at the `orchestration-service` (SVC-05) state machine level:

- **3DS-001**: The `PaymentIntent` state machine gains three new states: `ThreeDsRequired`, `ThreeDsAuthenticating`, `ThreeDsFailed`
- **3DS-002**: `connector-gateway` (SVC-04) gains `check_3ds_enrollment()` and `authenticate_3ds()` methods on the `AcquirerConnector` trait
- **3DS-003**: The checkout flow: Authorize → Check 3DS → If required → Redirect to ACS → Authenticate → Complete authorization
- **3DS-004**: 3DS exemptions supported: low-value, low-risk, trusted merchant, secure corporate, delegated, TRA

### 11.2 New: Circuit Breaker Pattern

Every `MerchantAcquirerLink` has an associated circuit breaker at `connector-gateway` (SVC-04) level:

- **CB-001**: Circuit breaker states: `Closed` (normal), `Open` (failing — skip), `HalfOpen` (testing recovery)
- **CB-002**: Opens when error rate exceeds configurable threshold (default: 50% failure in 30s window)
- **CB-003**: Stays open for configurable duration (default: 60s), then transitions to HalfOpen
- **CB-004**: In HalfOpen, limited requests allowed (default: 3). If all succeed → Close. If any fail → Open
- **CB-005**: Circuit breaker state affects routing: `orchestration-service` skips acquirers with open circuit breaker

### 11.3 New: BYOK (Bring Your Own Key) Model

The platform is a routing layer, NOT a payment gateway or payment facilitator:

- **BYOK-001**: Merchants bring their own merchant accounts with their own payment gateways
- **BYOK-002**: `merchant-acquirer-link-service` (SVC-21, NEW) manages the link lifecycle
- **BYOK-003**: Credentials are encrypte at rest via envelope encryption (KMS-managed KEK + per-link DEK)
- **BYOK-004**: Each connector defines an `OnboardingSchema` (dynamic credential form) rendered by the frontend
- **BYOK-005**: Credentials validated via sandbox/status-check before activation
- **BYOK-006**: Dual-key credential rotation supported (old + new credentials active during transition)

---

*End of Part 4. Proceed to Part 5: Payment Orchestration Engine (deep dive).*
