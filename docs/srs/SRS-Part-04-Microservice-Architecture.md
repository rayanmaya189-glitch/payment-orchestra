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

| Service ID | Service Name | Bounded Context(s) | Language/Runtime | Primary Datastore |
|---|---|---|---|---|
| SVC-01 | `tenant-service` | BC-01 Tenant Management | Rust (Axum + Tonic) | PostgreSQL |
| SVC-02 | `iam-service` | BC-02 Identity & Access | Rust | PostgreSQL + Redis (session/token cache) |
| SVC-03 | `compliance-service` | BC-03 Merchant Compliance (KYB) | Rust | PostgreSQL |
| SVC-04 | `connector-gateway` | BC-04 Gateway Connector Framework | Rust | PostgreSQL (connector config only; no transaction data) |
| SVC-05 | `orchestration-service` | BC-05 Payment Orchestration | Rust | PostgreSQL (event store) + Redis (idempotency cache, hot routing config) |
| SVC-06 | `invoice-service` | BC-06 Invoice Service | Rust | PostgreSQL |
| SVC-07 | `payment-link-service` | BC-07 Payment Link Service | Rust | PostgreSQL |
| SVC-08 | `subscription-service` | BC-08 Subscription Billing | Rust | PostgreSQL (event store) |
| SVC-09 | `reconciliation-service` | BC-09 Settlement & Reconciliation | Rust | PostgreSQL (event store) |
| SVC-10 | `dispute-service` | BC-10 Dispute Management | Rust | PostgreSQL (event store) |
| SVC-11 | `risk-service` | BC-11 Fraud & Risk Scoring | Rust | PostgreSQL + Redis (hot scoring cache) |
| SVC-12 | `ai-assistant-service` | BC-12 AI Payment Assistant | Rust (orchestrator) + Ollama (inference) | OpenSearch (vectors) + Postgres (conversation/citation metadata) |
| SVC-13 | `document-service` | BC-13 Document Management | Rust | PostgreSQL (metadata) + MinIO (blobs) |
| SVC-14 | `notification-service` | BC-14 Notification Service | Rust | PostgreSQL + Redis (delivery dedup) |
| SVC-15 | `analytics-service` | BC-15 Analytics & Reporting | Rust (ingestion) | ClickHouse |
| SVC-16 | `marketplace-service` | BC-16 Marketplace/Sub-Merchant | Rust | PostgreSQL |
| SVC-17 | `api-gateway` | Cross-cutting (not a bounded context) | Rust (Axum) | Redis (rate-limit counters only) |
| SVC-18 | `ai-gateway` | Cross-cutting routing/guardrail layer in front of SVC-12 | Rust | Redis (rate limits), Postgres (guardrail audit log) |

### 1.2 Deliberate Deviations from Strict 1:1 Mapping

- **SVC-17 (API Gateway) and SVC-18 (AI Gateway)** are cross-cutting infrastructure services with no owning bounded context from Part 3 — they exist for routing, authentication enforcement, rate limiting, and (for AI Gateway specifically) prompt/output guardrail enforcement. They own no domain data beyond operational counters/logs.
- **SVC-06 (Invoice) and SVC-07 (Payment Link)** are kept as separate deployable services despite being closely related in the domain model (§3.5, Part 3), because they have different scaling profiles (payment links are read-heavy/public-facing hosted pages; invoices are write-heavy/back-office) and different security postures (payment link hosted pages are unauthenticated public endpoints; invoice management is authenticated). Splitting them lets SVC-07 scale and be hardened independently without over-provisioning SVC-06.

---

## 2. API Gateway (SVC-17)

### 2.1 Responsibilities

- **GW-001**: Single ingress point for all external REST/gRPC-Web traffic (merchant dashboard, merchant server-to-server API calls, SDKs).
- **GW-002**: TLS termination, request authentication (validates JWT/API-key issued by SVC-02 IAM), and tenant-context extraction attached to every downstream request (enforcing PRIN-03 from Part 3 — no request reaches a domain service without a resolved tenant context).
- **GW-003**: Per-tenant and per-endpoint rate limiting (Redis-backed sliding window), protecting downstream services from noisy-neighbor tenants in the multi-tenant deployment.
- **GW-004**: Request routing to the correct backend service based on path/version (`/v1/payments/*` → SVC-05, `/v1/invoices/*` → SVC-06, etc.), with API versioning support (Part 10 defines the versioning policy in full).
- **GW-005**: Webhook signature verification pass-through configuration is NOT done here — inbound acquirer webhooks land on SVC-04 (`connector-gateway`) directly via dedicated, acquirer-specific signed endpoints, since each acquirer has a different signature/verification scheme that belongs in the ACL, not the generic gateway.

### 2.2 What the API Gateway Deliberately Does NOT Do

- No business logic, no domain validation beyond authentication/authorization gating — keeping GW-001–004 as its full responsibility set prevents it from becoming a second, undocumented home for domain rules that should live in Part 3's aggregates.

---

## 3. AI Gateway (SVC-18)

### 3.1 Why a Separate Gateway from the General API Gateway

The AI Gateway exists because AI-Assistant traffic has distinct requirements that would otherwise bloat SVC-17: prompt-injection guardrails, output-citation verification (BIZ-023), per-tenant AI-usage quota (a distinct commercial dimension from general API rate limits per BIZ-032), and routing to the correct model tier (Qwen3 32B for reasoning vs. Qwen3-VL 8B for document/vision tasks). Concentrating these concerns in SVC-18 keeps SVC-12 (`ai-assistant-service`) focused purely on RAG orchestration logic (detailed in Part 6).

### 3.2 Responsibilities

- **AIGW-001**: Route assistant requests to SVC-12; route document/vision-heavy requests specifically toward the Qwen3-VL 8B model pool vs. the Qwen3 32B reasoning pool (model selection policy, refined in Part 6).
- **AIGW-002**: Enforce per-tenant AI usage quotas/tiers (commercial model support, BIZ-032).
- **AIGW-003**: Apply input guardrails (basic prompt-injection pattern screening on user-supplied content that will be embedded in a RAG prompt, e.g., content pulled from uploaded documents) before it reaches the model — detailed guardrail design in Part 6.
- **AIGW-004**: Log every AI request/response pair (with citations) to the guardrail audit log (Postgres) for compliance review (BIZ-023, BIZ-040 adjacent).
- **AIGW-005**: Circuit-break to a graceful degradation response ("Assistant temporarily unavailable, here are the raw records") if the Ollama inference pool is unhealthy or over capacity, rather than queuing indefinitely and degrading the whole platform's perceived reliability.

---

## 4. Inter-Service Communication

### 4.1 Communication Style Decision Matrix

| Interaction | Style | Rationale |
|---|---|---|
| API Gateway → any domain service (synchronous user-facing request) | gRPC (internal), translated from REST/gRPC-Web at the edge | Low latency, strongly-typed contracts (Part 10), needed for checkout-path latency budgets (Part 11) |
| `orchestration-service` → `connector-gateway` (authorize/capture/refund call to acquirer) | gRPC (synchronous, in the checkout hot path) | Must return a result within the tenant's configured latency budget (BR-020-2, Part 2) |
| `orchestration-service` publishing domain events (EVT-01…EVT-12) | NATS JetStream (async, at-least-once, durable) | Multiple downstream consumers (BC-06, BC-08, BC-09, BC-12, BC-15 per Part 3 §4 table); publisher must not block on every consumer's processing speed |
| `reconciliation-service` ingesting settlement files/webhooks | Mixed: webhook ingress via gRPC/REST at connector-gateway → published as NATS event; polling/SFTP-based ingestion is a scheduled job publishing directly to NATS | Decouples ingestion cadence (which varies wildly per acquirer) from downstream reconciliation processing |
| `ai-assistant-service` reading context from other contexts | Direct read-model queries (gRPC query endpoints exposed by owning services, or direct read-only access to each service's dedicated read-replica/projection — never the write-model database) | BC-12 is a read-only Conformist (Part 3 §1.3); it must never acquire a write path into other contexts |
| `notification-service` triggering | NATS JetStream subscription to relevant events (EVT-03, EVT-07, EVT-15, EVT-17, etc.) | Naturally asynchronous, at-least-once with idempotent send-dedup (Part 3 §5.10) |

### 4.2 NATS JetStream Subject Taxonomy

Following the naming convention introduced in Part 3 §4:

```
events.<context>.<aggregate>.<event_name>.v<version>

Examples:
events.orchestration.payment_intent.payment_authorized.v1
events.reconciliation.settlement_batch.settlement_record_matched.v1
events.dispute.chargeback_case.chargeback_received.v1
```

- **Streams**: One JetStream stream per bounded context (e.g., `ORCHESTRATION_EVENTS`, `RECONCILIATION_EVENTS`), partitioned by subject wildcard, retained per the policy resolved in OQ-008 (Part 3) — to be finalized numerically in Part 9 alongside storage sizing.
- **Consumer groups**: Each downstream service creates a durable consumer per stream it subscribes to, with explicit ack after successful projection/side-effect processing, and dead-letter handling (redeliver with backoff, then park in a `*_DLQ` subject after N failed attempts) surfaced to SVC-07... correction: surfaced to an operational alert channel monitored by ACT-07 (Support/Ops Engineer, Part 2).
- **Exactly-once processing semantics**: NATS JetStream provides at-least-once delivery; **exactly-once effect** is achieved at the consumer level via idempotent projections keyed on `event_id` (dedup table per consumer), not assumed from the transport layer.

### 4.3 gRPC Contract Ownership

Each domain service publishes its own `.proto` service definition (full contracts in Part 10); `connector-gateway` additionally defines the internal `AcquirerConnector` gRPC-equivalent trait contract (Rust trait, Part 7) that concrete acquirer adapters implement in-process (not a network call per adapter — adapters are compiled into `connector-gateway` as plugins/modules, not separate microservices per acquirer, to avoid N-times deployment overhead for what are essentially stateless protocol translators).

---

## 5. Service-by-Service Responsibility Detail

*(Deep internals for SVC-05, SVC-04, SVC-12 are deferred to their dedicated Parts 5, 7, 6 respectively; this section covers the remaining services not given a dedicated part.)*

### 5.1 SVC-01 `tenant-service`
- Owns tenant registration (UC-001), tenant status lifecycle, tenant member list (before fine-grained roles, which SVC-02 owns).
- Exposes: `CreateTenant`, `GetTenant`, `UpdateTenantStatus` (internal-only, called by `compliance-service` on KYB approval), `ListTenantMembers`.
- Publishes: `TenantRegistered`, `TenantVerified`, `TenantSuspended`.

### 5.2 SVC-02 `iam-service`
- Owns authentication (issuing/validating JWTs), RBAC role definitions, ABAC attribute conditions (OQ-006 threshold-based approval rules).
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

### 5.13 SVC-16 `marketplace-service`
- Consulted synchronously by `orchestration-service` at routing time (via `SplitConfigurationActivated` state, cached) when a transaction is flagged as a marketplace/split transaction; integrates with the external licensed split-disbursement partner via its own ACL.

---

## 6. Scheduling & Background Jobs

- **JOB-001**: Subscription renewal triggers (SVC-08, internal cron per tenant billing cycle).
- **JOB-002**: Dunning retry execution (SVC-08).
- **JOB-003**: Settlement file polling for acquirers that don't support webhook push (SVC-09, via `connector-gateway` polling adapters).
- **JOB-004**: Invoice overdue transition + reminder trigger (SVC-06).
- **JOB-005**: AI Assistant retrieval-index refresh/compaction (SVC-12, periodic re-embedding of updated documents — detailed in Part 6).
- **JOB-006**: Reconciliation exception aging alerts (SVC-09 → SVC-14).

For MVP, scheduling is implemented as in-process cron-style schedulers within the owning service (simplicity, avoids introducing a separate distributed scheduler service prematurely); if job volume/complexity grows past H1, a dedicated scheduling service is a candidate future extraction — explicitly flagged here as a deferred architectural decision, not a gap.

---

## 7. Multi-Tenancy Enforcement at the Service Layer

- **MT-001**: Every gRPC request (internal, service-to-service) carries a `tenant_context` metadata field populated by API Gateway (§2) at ingress and propagated unchanged through every downstream hop — no service is permitted to "look up" a tenant from a body field for authorization purposes; only the propagated, gateway-verified context is trusted.
- **MT-002**: Every database query in every service includes `tenant_id` in its `WHERE` clause via a shared Rust data-access macro/pattern (detailed in Part 9) — code review and automated lint rules (Part 11) enforce that no raw query bypasses this pattern.
- **MT-003**: NATS subjects do not need per-tenant subject partitioning at the transport level (all tenants share the same subject namespace, e.g., `events.orchestration.payment_intent.payment_authorized.v1`), but every event payload's envelope (Part 3 §4) carries `tenant_id`, and every consumer's projection logic enforces tenant scoping when writing to its own store (defense in depth: transport-level sharing is fine because storage-level isolation is enforced downstream).

---

## 8. Deployment Topology (Preview)

*(Full Kubernetes/Docker deployment specification is in Part 11; this section previews the shape so Parts 5–10 can assume a consistent mental model.)*

- Each service in §1.1 is an independently deployable, independently scalable container. `orchestration-service` and `connector-gateway` are provisioned with the highest replica-count floor and tightest autoscaling responsiveness, since they sit on the checkout-latency-critical path.
- `ai-assistant-service` and its Ollama inference backend are deployed on GPU-backed node pools, separate from the general CPU-only service mesh, with the `ai-gateway` mediating so that a spike in AI usage cannot starve GPU resources needed for anything else (there is nothing else GPU-bound at MVP, but this isolation is kept as a forward-looking discipline).
- All services are deployed behind a service mesh providing mTLS between services (Part 8, zero-trust internal networking) — this is what makes MT-001's "trust only the propagated gateway context" claim enforceable rather than aspirational: services physically cannot be reached except through authenticated mesh identities.

---

## 9. Traceability to Part 1 / Part 2 / Part 3

| Requirement/Context | Realized By |
|---|---|
| BIZ-030 (tenant isolation) | §7 MT-001/002/003 |
| BIZ-021 (self-hosted AI) | §3, §8 GPU-isolated `ai-assistant-service` deployment |
| BIZ-023 (citable AI answers) | §3.2 AIGW-004 guardrail audit log |
| BR-020-2 (bounded failover latency) | §4.1 synchronous gRPC choice for orchestration↔connector-gateway |
| PRIN-03 (Part 3, structural tenant scoping) | §7 MT-002 |
| BC-12 read-only Conformist (Part 3 §1.3) | §4.1 "direct read-model queries... never the write-model database" |

---

## 10. Open Items Carried Forward

- **OQ-009**: Confirm whether `risk-service` (SVC-11) synchronous scoring call adds unacceptable latency to the checkout path at MVP — needs a benchmark once Part 11 performance targets are set; if too slow, MVP may ship with routing-time risk scoring disabled by default and enabled per-tenant opt-in.
- **OQ-010**: Decide whether `invoice-service` and `payment-link-service` (SVC-06/07) should share a single Postgres instance (different schemas) or fully separate instances at MVP scale — cost vs. isolation trade-off to be resolved in Part 9.

---

*End of Part 4. Proceed to Part 5: Payment Orchestration Engine (deep dive).*
