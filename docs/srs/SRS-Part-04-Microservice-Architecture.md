# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

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
| SVC-04 | `connector-gateway` | BC-04 Gateway Connector Framework | Rust (SeaORM) | SeaORM | PostgreSQL (connector config only; no transaction data) |
| SVC-05 | `orchestration-service` | BC-05 Payment Orchestration | Rust (SeaORM) | SeaORM | PostgreSQL (event store) + Redis (idempotency cache, hot routing config) |
| SVC-06 | `invoice-service` | BC-06 Invoice Service | Rust (SeaORM) | SeaORM | PostgreSQL |
| SVC-07 | `payment-link-service` | BC-07 Payment Link Service | Rust (SeaORM) | SeaORM | PostgreSQL |
| SVC-08 | `subscription-service` | BC-08 Subscription Billing | Rust (SeaORM) | SeaORM | PostgreSQL (event store) |
| SVC-09 | `reconciliation-service` | BC-09 Settlement & Reconciliation | Rust (SeaORM) | SeaORM | PostgreSQL (event store) |
| SVC-10 | `dispute-service` | BC-10 Dispute Management | Rust (SeaORM) | SeaORM | PostgreSQL (event store) |
| SVC-11 | `risk-service` | BC-11 Fraud & Risk Scoring | Rust (SeaORM) | SeaORM | PostgreSQL + Redis (hot scoring cache) |
| SVC-12 | `ai-assistant-service` | BC-12 AI Payment Assistant | Rust (SeaORM) | SeaORM | OpenSearch (vectors) + Postgres (conversation/citation metadata) |
| SVC-13 | `document-service` | BC-13 Document Management | Rust (SeaORM) | SeaORM | PostgreSQL (metadata) + MinIO (blobs) |
| SVC-14 | `notification-service` | BC-14 Notification Service | Rust (SeaORM) | SeaORM | PostgreSQL + Redis (delivery dedup) |
| SVC-15 | `analytics-service` | BC-15 Analytics & Reporting | Rust | ClickHouse driver | ClickHouse |
| SVC-17 | `api-gateway` | Cross-cutting (not a bounded context) | Rust (Axum) | SeaORM | Redis (rate-limit counters only) |
| SVC-18 | `ai-gateway` | Cross-cutting routing/guardrail layer in front of SVC-12 | Rust (Axum) | SeaORM | Redis (rate limits), Postgres (guardrail audit log) |

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

- No business logic, no domain validation beyond authentication/authorization gating — keeping GW-001–004 as its full responsibility set prevents it from becoming a second, undocumented home for domain rules that should live in Part 3's aggregates.

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
| API Gateway → domain service (synchronous user-facing request) | **gRPC** | HTTP/2 + Protobuf | Low latency, strongly-typed contracts (Part 10), needed for checkout-path latency budgets (Part 11) |
| `orchestration-service` → `connector-gateway` (authorize/capture/refund call to acquirer) | **gRPC** (synchronous, in the checkout hot path) | HTTP/2 + Protobuf | Must return a result within the latency budget (BR-020-2, Part 2) |
| `orchestration-service` → `risk-service` (pre-authorization risk score) | **gRPC** (synchronous, in the checkout hot path) | HTTP/2 + Protobuf | Risk score needed before routing decision; must be fast |
| `orchestration-service` publishing domain events (EVT-01…EVT-19) | **NATS JetStream** (async, durable) | NATS protocol | Multiple downstream consumers; publisher must not block |
| `reconciliation-service` ingesting settlement files | **Mixed**: webhook via gRPC at connector-gateway → published to NATS; polling/SFTP via scheduled job → published to NATS | NATS protocol | Decouples ingestion cadence from downstream processing |
| `ai-assistant-service` reading context from other contexts | **gRPC** (direct read-model queries) | HTTP/2 + Protobuf | BC-12 is a read-only Conformist; must never acquire write path |
| `notification-service` triggering | **NATS JetStream** subscription to relevant events | NATS protocol | Naturally asynchronous, at-least-once with idempotent dedup |
| `compliance-service` → `document-service` (OCR trigger) | **gRPC** (synchronous) | HTTP/2 + Protobuf | Request-response; caller needs OCR result |
| `analytics-service` consuming events | **NATS JetStream** subscription to all event streams | NATS protocol | Pure event consumer, never called synchronously |
| `invoice-service` / `subscription-service` state updates | **NATS JetStream** subscription to orchestration events | NATS protocol | Eventually consistent; invoice/subscription status updated async after payment events |
| Cross-service queries (read-heavy, non-critical path) | **gRPC** (query endpoints) | HTTP/2 + Protobuf | Type-safe, observable, traceable |

### 4.2 gRPC vs NATS Decision Rules

- **RULE-001**: Use **gRPC** when the caller needs a synchronous response within the request's latency budget (checkout path, pre-authorization checks, document OCR requests).
- **RULE-002**: Use **NATS JetStream** when the interaction is naturally asynchronous, fan-out to multiple consumers, or doesn't require the caller to wait for completion (domain event publishing, notification dispatch, analytics ingestion).
- **RULE-003**: Never use NATS for the checkout hot path (Create/Authorize/Capture) — the latency of NATS publish-ack is unnecessary overhead when a direct gRPC call is more appropriate.
- **RULE-004**: Never use gRPC for fan-out event distribution — the publisher would need to know and manage all consumers, defeating the decoupling benefit of event-driven architecture.
- **RULE-005**: All service-to-service calls use gRPC with Protobuf — SeaORM services generate clients from `.proto` files for type-safe inter-service communication.

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
- MVP: synchronous rule-based scoring call from `orchestration-service` before authorization (low-latency requirement, Redis-cached rule set). H3: asynchronous ML scoring feeding back into routing decisions (GOAL-009 adjacent) — architecture must not preclude this evolution (extensibility NFR, Part 8).

### 5.10 SVC-13 `document-service`
- Owns MinIO-backed blob storage abstraction and metadata; triggers OCR pipeline (Qwen3-VL 8B, via `ai-gateway`/`ai-assistant-service`) asynchronously on upload, publishes `DocumentOcrCompleted` with extracted structured fields for the calling context (e.g., `compliance-service`) to consume.

### 5.11 SVC-14 `notification-service`
- Subscribes broadly across event streams per §4.1; owns templated email/SMS/push dispatch; is the only service permitted to call external notification providers (email/SMS gateways) — no other service sends customer-facing notifications directly, to keep delivery auditing (BIZ-040-adjacent) centralized.

### 5.12 SVC-15 `analytics-service`
- Pure event consumer across effectively all streams; writes append-only into ClickHouse; exposes read-only query endpoints for dashboards (UC-070) and report export (UC-071). Never receives direct write commands from users — all its data is derived.

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

For MVP, scheduling is implemented as in-process cron-style schedulers within the owning service; if job volume/complexity grows past H1, a dedicated scheduling service is a candidate future extraction.

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

### 7.1 Event Replay Mechanism

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
- `ai-assistant-service` and its Ollama inference backend are deployed on GPU-backed node pools, separate from the general CPU-only service mesh, with the `ai-gateway` mediating so that a spike in AI usage cannot starve GPU resources needed for anything else (there is nothing else GPU-bound at MVP, but this isolation is kept as a forward-looking discipline).
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

## 10. Open Items Carried Forward

- **OQ-009**: Confirm whether `risk-service` (SVC-11) synchronous scoring call adds unacceptable latency to the checkout path at MVP — needs a benchmark once Part 11 performance targets are set; if too slow, MVP may ship with routing-time risk scoring disabled by default and enabled per-operator opt-in.
- **OQ-010**: Decide whether `invoice-service` and `payment-link-service` (SVC-06/07) should share a single Postgres instance (different schemas) or fully separate instances at MVP scale — cost vs. isolation trade-off to be resolved in Part 9.

---

*End of Part 4. Proceed to Part 5: Payment Orchestration Engine (deep dive).*
