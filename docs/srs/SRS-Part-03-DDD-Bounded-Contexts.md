# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 3 of 12:** Domain-Driven Design, Bounded Contexts, Aggregates & Domain Events
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 3 of 12 — DDD & Bounded Contexts |
| Depends On | Part 1 (Business Requirements), Part 2 (Use Cases) |
| Feeds Into | Part 4 (Microservice Architecture — one microservice per bounded context, generally), Part 5 (Payment Orchestration deep-dive), Part 6 (AI Assistant deep-dive), Part 9 (Database Design), Part 10 (API/gRPC contracts) |
| ID Scheme | `BC-##` (bounded context), `AGG-##` (aggregate), `EVT-##` (domain event), `INV-##` (invariant) |

### 0.1 Modeling Approach

This platform is modeled using **strategic DDD** (context mapping, ubiquitous language per context, explicit anti-corruption layers at every external-system boundary) and **tactical DDD** (aggregates, entities, value objects, domain events, repositories) implemented via **event sourcing + CQRS** for money-movement-relevant contexts (Payment Orchestration, Settlement/Reconciliation, Dispute Management) and simpler CRUD-plus-events for lower-risk supporting contexts (Notification, Document Management metadata).

**ORM Layer**: All database access is through ORMs — no raw SQL in application code:
- **Rust services** (orchestration, connector-gateway, AI assistant, risk): SeaORM entities with derive macros
- **Go services** (tenant, IAM, compliance, notifications, analytics, document): Ent ORM schemas with generated code

This distinction is deliberate — not every context needs the cost of full event sourcing, and applying it uniformly would be over-engineering exactly the contexts (e.g., Notification templates) where it adds no auditability value.

**Why event sourcing for the core money-movement contexts specifically**: BIZ-040 (Part 1) requires immutable, complete audit trails of every money-movement-relevant event. Event sourcing makes "what happened and in what order" the source of truth by construction, rather than a derived/logged side effect of CRUD updates — which directly satisfies BIZ-040 and SUCC-005 without a separate audit subsystem bolted on afterward.

**Why UUIDv7 over ULID or UUIDv4**: UUIDv7 (RFC 9562) was chosen over ULID because UUIDv7 is an IETF standard with broad ecosystem support across both Rust (`uuid` crate) and Go (`google/uuid`), whereas ULID is a community specification with less consistent library support. UUIDv7 was chosen over UUIDv4 because UUIDv4 is random and causes B-tree index fragmentation on high-throughput tables (event_store, outbox) — UUIDv7's timestamp prefix provides sequential insert order, dramatically improving write performance and reducing index bloat.

---

## 1. Strategic Design: Context Map

### 1.1 Bounded Context Catalog (Overview)

| ID | Bounded Context | Type | Consistency Model |
|---|---|---|---|
| BC-01 | Operator Management | Core Supporting | Strongly consistent (Postgres, transactional) |
| BC-02 | Identity & Access (IAM) | Generic Supporting | Strongly consistent |
| BC-03 | Merchant Compliance (KYB) | Core Supporting | Strongly consistent + async partner integration |
| BC-04 | Gateway Connector Framework | Core (Anti-Corruption Layer) | Strongly consistent, adapter pattern |
| BC-05 | Payment Orchestration | **Core Domain** | Event-sourced, strongly consistent per aggregate, eventually consistent read models |
| BC-06 | Invoice Service | Core Supporting | Strongly consistent, references BC-05 |
| BC-07 | Payment Link Service | Core Supporting | Strongly consistent, thin layer over BC-05 |
| BC-08 | Subscription Billing | Core Supporting | Event-sourced (billing history matters for disputes) |
| BC-09 | Settlement & Reconciliation | **Core Domain** | Event-sourced, eventually consistent against BC-05 |
| BC-10 | Dispute Management (Chargebacks) | Core Supporting | Event-sourced |
| BC-11 | Fraud & Risk Scoring | Core Supporting | Eventually consistent, read-heavy |
| BC-12 | AI Payment Assistant (RAG) | **Core Domain (differentiator)** | Eventually consistent, read-only over other contexts |
| BC-13 | Document Management | Generic Supporting | Strongly consistent metadata, object storage for blobs |
| BC-14 | Notification Service | Generic Supporting | Eventually consistent, at-least-once delivery |
| BC-15 | Analytics & Reporting | Generic Supporting | Eventually consistent (ClickHouse), append-only |
| BC-17 | Saga Coordinator | Cross-Cutting Infrastructure | Durable state machine, Postgres-backed |

### 1.2 Context Map — Relationships

Using standard DDD context-mapping patterns (Partnership, Customer/Supplier, Conformist, Anti-Corruption Layer, Shared Kernel, Open Host Service/Published Language):

```
 [BC-04 Gateway Connector Framework] ──ACL──> External Acquirers/PSPs (ACT-08)
              │  (Open Host Service: normalized Authorize/Capture/Refund/Void API)
              ▼
 [BC-05 Payment Orchestration] <──Customer/Supplier── [BC-06 Invoice Service]
              │        ▲                               [BC-07 Payment Link Service]
              │        │                                [BC-08 Subscription Billing]
              │        │ (Published Language: Domain Events, §4)
              ▼        │
 [BC-09 Settlement & Reconciliation] ──Customer/Supplier──> [BC-15 Analytics & Reporting]
              │
              ▼
 [BC-10 Dispute Management] ──reads events from── [BC-05, BC-09]

 [BC-12 AI Payment Assistant] ──Conformist (read-only consumer)──> [BC-05, BC-09, BC-10, BC-13, BC-15]
              │ (never writes back into other contexts directly — all AI-suggested
              │  actions go through the normal command API of the owning context,
              │  requiring human confirmation per BR-041-1, Part 2)
              ▼
        (no outbound writes except via owning context's command API)

 [BC-01 Operator Management] ── provides identity context ── ALL contexts
 [BC-02 Identity & Access]  ── provides authentication/authorization context ── ALL contexts
 [BC-03 Merchant Compliance] ──ACL──> External KYB Partner API
```
 [BC-13 Document Management] ── Open Host Service (upload/fetch/OCR-trigger API) ── BC-03, BC-09, BC-12
 [BC-14 Notification Service] ── Open Host Service (send notification API) ── BC-05, BC-06, BC-08, BC-10
```

### 1.3 Rationale for Context Boundaries

- **BC-04 (Gateway Connector Framework) is deliberately separated from BC-05 (Payment Orchestration)** even though they are tightly related, because BC-04's entire reason to exist is translating N different acquirer APIs into one normalized internal protocol (the Anti-Corruption Layer pattern). Merging them would leak acquirer-specific concepts (e.g., a specific PSP's proprietary decline code taxonomy) into the core orchestration domain model, violating BIZ-010's requirement that routing be configurable without code change per acquirer.
- **BC-09 (Settlement & Reconciliation) is separate from BC-05 (Payment Orchestration)** because they have fundamentally different temporal characteristics: orchestration is synchronous/near-real-time (seconds), reconciliation is batch/asynchronous (settlement files arrive hours to days later) and reasons over a different aggregate root (`SettlementBatch` vs `PaymentIntent`). Conflating them would force the orchestration hot path to carry reconciliation-batch complexity it doesn't need.
- **BC-12 (AI Payment Assistant) is modeled as a read-only Conformist** against every other context specifically so that the "no unauthorized AI-driven money movement" guardrail (BR-041-1, BR-050-1 from Part 2) is a *structural* property of the architecture, not merely a prompt-level instruction to the model. The AI service has no command API for money-movement contexts in its dependency graph — this is enforced at the network/service-mesh level in Part 4, not just documented here.
- **BC-12 (AI Payment Assistant) is modeled as a read-only Conformist** against every other context specifically so that the "no unauthorized AI-driven money movement" guardrail is a *structural* property of the architecture, not merely a prompt-level instruction to the model.

---

## 2. Ubiquitous Language Glossary (Selected Core Terms)

A full glossary appendix will be assembled in Part 12; the terms below are those most load-bearing for later parts and must not be used inconsistently across the document series.

| Term | Definition | Owning Context |
|---|---|---|
| **Operator** | The registered organization (merchant or platform operator) using the platform. | BC-01 |
| **Merchant Acquirer Link** | A configured, credentialed connection between the operator and a specific acquirer/PSP. | BC-04 |
| **Routing Policy** | The active, versioned set of rules determining which acquirer(s) a `PaymentIntent` is routed to, and in what fallback order. | BC-05 |
| **Payment Intent** | The aggregate root representing a single attempted payment through its full lifecycle (Created → Authorized → Captured/Failed/Refunded). | BC-05 |
| **Settlement Record** | A normalized representation of a single settled-transaction line from an acquirer's settlement file/webhook. | BC-09 |
| **Settlement Batch** | The aggregate root representing one ingested settlement file/batch and its matching outcome against `PaymentIntent`s. | BC-09 |
| **Reconciliation Exception** | A `SettlementRecord` that could not be automatically matched to a `PaymentIntent`. | BC-09 |
| **Chargeback Case** | The aggregate root tracking a dispute from receipt through representment to final outcome. | BC-10 |
| **Grounding Context (RAG)** | The retrieved set of documents/events assembled to ground an AI Assistant answer. | BC-12 |
| **Custody** | Legal/economic control over funds. The platform, by design (Part 1 §6), never acquires custody of merchant/customer funds in any bounded context. | Cross-cutting (Part 1) |

---

## 3. Tactical Design: Core Bounded Contexts in Detail

For each core-domain bounded context, this section defines: purpose, aggregates (with entities and value objects), invariants, and the domain events it publishes. Supporting/generic contexts are covered more briefly in §5.

### 3.1 BC-05 — Payment Orchestration (THE Core Domain)

**Purpose**: Own the full lifecycle of a payment attempt, apply routing/failover policy, and interface with BC-04 to reach external acquirers, while never taking custody of funds (Part 1 §6).

#### AGG-01: `PaymentIntent` (Aggregate Root)

- **Entities**:
  - `PaymentIntent` (root) — identity: `payment_intent_id` (UUIDv7).
  - `RoutingAttempt` (entity, child of `PaymentIntent`) — one per acquirer hop attempted (supports failover history, EX-020b idempotency safeguard from Part 2).
- **Value Objects**:
  - `Money` (amount: integer minor units, currency: ISO 4217 code) — always integer minor units internally to avoid floating-point rounding defects (a hard engineering rule, not a suggestion).
  - `IdempotencyKey` (tenant-scoped, caller-supplied, required on every mutating command per BR-020-1, Part 2).
  - `AcquirerReference` (acquirer-specific transaction reference string, opaque to the domain model — this is the ACL boundary artifact from BC-04).
  - `DeclineReason` (normalized enum, mapped from acquirer-specific codes by BC-04's ACL — never the raw acquirer code, so routing rules in BC-05 stay acquirer-agnostic per BIZ-010).
- **State Machine** (simplified): `Created → Authorizing → Authorized → Capturing → Captured → [Refunding → Refunded/PartiallyRefunded]`, with `Authorizing`/`Capturing` able to transition to `Failed` (with terminal `FailedAllRoutes` if every routing hop exhausted) and any authorized-but-uncaptured state able to transition to `Voided` or `AuthorizationExpired`.
- **Invariants**:
  - **INV-01**: A `PaymentIntent` can only be captured for an amount less than or equal to its authorized amount, and only once per authorization (unless partial-capture is explicitly supported by the connector and configured — tracked via a running `captured_amount` that must never exceed `authorized_amount`).
  - **INV-02**: A `PaymentIntent`'s `RoutingAttempt` list must never contain two `Authorized`-outcome attempts simultaneously (double-authorization prevention, ties to EX-020b in Part 2).
  - **INV-03**: Refunds against a `PaymentIntent` must always target the same acquirer that captured it (BR-022-1, Part 2), enforced at the aggregate level, not merely at the API layer.
  - **INV-04**: Every state transition must be caused by exactly one recorded domain event; the aggregate's current state is always a pure fold over its event stream (event-sourcing discipline).
- **Domain Events published** (see also full catalog in §4):
  - `EVT-01 PaymentIntentCreated`
  - `EVT-02 PaymentAuthorizationAttempted` (one per `RoutingAttempt`, including failed attempts — needed for GOAL-002 revenue-recovery measurement and AI Assistant anomaly grounding)
  - `EVT-03 PaymentAuthorized`
  - `EVT-04 PaymentCaptured` / `EVT-05 PaymentPartiallyCaptured`
  - `EVT-06 PaymentFailed` (single-hop) / `EVT-07 PaymentFailedAllRoutes` (terminal)
  - `EVT-08 PaymentVoided`
  - `EVT-09 PaymentRefunded` / `EVT-10 PaymentPartiallyRefunded`

#### AGG-02: `RoutingPolicy` (Aggregate Root)

- **Entities**: `RoutingRule` (ordered, versioned child entities).
- **Value Objects**: `RoutingCondition` (card scheme, currency, amount-threshold predicates), `FailoverConfig` (retryable decline codes, max hops, latency budget).
- **Invariants**:
  - **INV-05**: A `RoutingPolicy` version is immutable once activated; changes create a new version (never in-place mutation), so historical `PaymentIntent`s can be audited against the exact policy version that governed them (BR-011-1, Part 2).
- **Domain Events**: `EVT-11 RoutingPolicyActivated`, `EVT-12 RoutingPolicyDeactivated`.

### 3.2 BC-09 — Settlement & Reconciliation

#### AGG-03: `SettlementBatch` (Aggregate Root)

- **Entities**: `SettlementRecord` (child entities, one per settled transaction line within the batch).
- **Value Objects**: `SettlementFileChecksum`, `SettlementMatchOutcome` (Matched / Unmatched / AmountMismatch).
- **Invariants**:
  - **INV-06**: A `SettlementBatch` cannot be re-ingested if its checksum matches a previously ingested batch (idempotent ingestion, BR-040-1, Part 2).
  - **INV-07**: A `SettlementRecord` in `Matched` state must reference exactly one `PaymentIntent` (from BC-05) and the matched amount must be recorded even if it differs from the original captured amount (to make fee deductions and FX differences visible rather than silently reconciled away).
- **Domain Events**: `EVT-13 SettlementBatchIngested`, `EVT-14 SettlementRecordMatched`, `EVT-15 SettlementRecordUnmatched`, `EVT-16 ReconciliationExceptionResolved`.

### 3.3 BC-10 — Dispute Management

#### AGG-04: `ChargebackCase` (Aggregate Root)

- **Entities**: `RepresentmentSubmission` (child entity per evidence submission attempt).
- **Value Objects**: `ChargebackReasonCode` (normalized per scheme via BC-04 ACL), `ChargebackOutcome`.
- **Invariants**: **INV-08**: A `ChargebackCase` must always reference exactly one `PaymentIntent` and cannot be created for a `PaymentIntent` that was never `Captured`.
- **Domain Events**: `EVT-17 ChargebackReceived`, `EVT-18 RepresentmentSubmitted`, `EVT-19 ChargebackResolved`.

---

## 4. Domain Event Catalog (Consolidated)

All domain events are versioned, immutable, tenant-scoped, and published to NATS JetStream subjects following the naming convention `events.<bounded_context>.<aggregate>.<event_name>.v<version>` (full subject taxonomy in Part 4). Every event carries a common envelope:

```
EventEnvelope {
  event_id: UUIDv7
  aggregate_type: string
  aggregate_id: UUIDv7
  event_type: string
  event_version: u16
  occurred_at: timestamp (UTC)
  actor: ActorReference (user_id | system_actor_id)
  causation_id: UUIDv7        // the command that caused this event
  correlation_id: UUIDv7      // ties together a full business transaction across contexts
  payload: bytes (protobuf-encoded, schema per event type — Part 10)
}
```

| Event ID | Event Name | Publishing Context | Consumed By |
|---|---|---|---|
| EVT-01 | PaymentIntentCreated | BC-05 | BC-09, BC-15, BC-12 |
| EVT-02 | PaymentAuthorizationAttempted | BC-05 | BC-15, BC-12, BC-11 |
| EVT-03 | PaymentAuthorized | BC-05 | BC-06, BC-08, BC-09, BC-14 |
| EVT-04 | PaymentCaptured | BC-05 | BC-06, BC-08, BC-09, BC-15 |
| EVT-05 | PaymentPartiallyCaptured | BC-05 | BC-09, BC-15 |
| EVT-06 | PaymentFailed | BC-05 | BC-15, BC-12, BC-11, BC-14 |
| EVT-07 | PaymentFailedAllRoutes | BC-05 | BC-14, BC-12, BC-11 |
| EVT-08 | PaymentVoided | BC-05 | BC-06, BC-09 |
| EVT-09 | PaymentRefunded | BC-05 | BC-06, BC-09, BC-14 |
| EVT-10 | PaymentPartiallyRefunded | BC-05 | BC-09, BC-14 |
| EVT-11 | RoutingPolicyActivated | BC-05 | BC-15 (audit), BC-12 |
| EVT-12 | RoutingPolicyDeactivated | BC-05 | BC-15 (audit) |
| EVT-13 | SettlementBatchIngested | BC-09 | BC-15 |
| EVT-14 | SettlementRecordMatched | BC-09 | BC-06, BC-08, BC-15 |
| EVT-15 | SettlementRecordUnmatched | BC-09 | BC-14 (alert), BC-12 |
| EVT-16 | ReconciliationExceptionResolved | BC-09 | BC-15 |
| EVT-17 | ChargebackReceived | BC-10 | BC-14, BC-15, BC-12 |
| EVT-18 | RepresentmentSubmitted | BC-10 | BC-15 |
| EVT-19 | ChargebackResolved | BC-10 | BC-06 (funds impact note), BC-15 |

*(Supporting-context events — Tenant lifecycle, IAM role changes, KYB status changes, notification delivery, document upload/OCR completion — are cataloged in §5 alongside their owning contexts, to keep this table focused on money-movement-relevant events per BIZ-040's audit priority.)*

---

## 5. Supporting & Generic Bounded Contexts (Brief)

### 5.1 BC-01 — Operator Management
- **Aggregate**: `Operator` (root), entities: `OperatorMember`. Events: `OperatorRegistered`, `OperatorVerified`, `OperatorSuspended`.
- Owns operator identity used across all contexts (single-tenant, so operator context is implicit but still modeled for lifecycle management).

### 5.2 BC-02 — Identity & Access (IAM)
- **Aggregate**: `Principal` (root — represents a human user or a service account), entities: `RoleAssignment`. Value objects: `Permission`, `Role` (RBAC), `AttributeCondition` (ABAC, e.g., "can approve reconciliation exceptions only up to X amount" — ties to OQ-006, Part 2).
- Events: `PrincipalCreated`, `RoleAssigned`, `PermissionDenied` (yes, denials are also events — required for security audit per Part 8).

### 5.3 BC-03 — Merchant Compliance (KYB)
- **Aggregate**: `KybCase` (root). Events: `KybCaseSubmitted`, `KybCaseApproved`, `KybCaseRejected`.
- Anti-corruption layer against external KYB partner APIs — the domain model represents partner decisions as opaque `PartnerDecision` value objects, never assuming a specific partner's internal risk-scoring taxonomy leaks into the core model.

### 5.4 BC-04 — Gateway Connector Framework
- Not modeled as a traditional aggregate-owning context in the DDD sense — it is primarily an **Anti-Corruption Layer + Open Host Service**. Its core abstraction is the `AcquirerConnector` trait (Rust trait, detailed in Part 7) that every acquirer-specific adapter implements, normalizing to BC-05's `AcquirerReference`/`DeclineReason` value objects.

### 5.5 BC-06/BC-07 — Invoice Service / Payment Link Service
- **Aggregates**: `Invoice` (root, entities: `InvoiceLineItem`), `PaymentLink` (root). Both reference `PaymentIntent`(s) from BC-05 by ID (never duplicate payment state). Events: `InvoiceCreated`, `InvoiceSent`, `InvoicePaid`, `InvoiceOverdue`, `PaymentLinkCreated`, `PaymentLinkExpired`.

### 5.6 BC-08 — Subscription Billing
- **Aggregate**: `Subscription` (root), entities: `BillingCycle`. Event-sourced given dispute-relevance of full billing history. Events: `SubscriptionCreated`, `SubscriptionRenewed`, `SubscriptionRenewalFailed`, `SubscriptionDunningExhausted`, `SubscriptionCancelled`.

### 5.7 BC-11 — Fraud & Risk Scoring
- **Aggregate**: `RiskAssessment` (root, linked 1:1 with a `PaymentIntent`). MVP is rule-based (heuristic scoring); H3 introduces ML-based scoring (GOAL-009 adjacent). Events: `RiskAssessmentCompleted`, `TransactionFlaggedHighRisk`.

### 5.8 BC-12 — AI Payment Assistant (RAG)
- Not an aggregate-owning transactional context; modeled as a **query-side, read-only context** with its own internal state limited to: `ConversationSession`, `GroundingCitation` records (for audit per BIZ-023), and retrieval indices (BGE-M3 embeddings + OpenSearch). Full design in Part 6.

### 5.9 BC-13 — Document Management
- **Aggregate**: `DocumentRecord` (root, metadata only — blobs live in MinIO). Events: `DocumentUploaded`, `DocumentOcrCompleted`, `DocumentOcrFailed`.

### 5.10 BC-14 — Notification Service
- **Aggregate**: `NotificationRequest` (root). Events: `NotificationQueued`, `NotificationSent`, `NotificationFailed`. At-least-once delivery semantics (Part 4) — consumers of notification-triggering events must be idempotent to duplicate sends.

### 5.11 BC-15 — Analytics & Reporting
- Not aggregate-owning — a pure **query-side read model** built by consuming the domain events in §4 into ClickHouse (append-only, columnar). No commands originate here.

---

## 6. Aggregate Design Principles (Cross-Cutting Rules)

- **PRIN-01 (Consistency boundary = transaction boundary)**: Each aggregate is the sole authority for its own invariants; a single command can mutate exactly one aggregate instance transactionally. Cross-aggregate effects happen via domain events consumed asynchronously (never a distributed transaction spanning two aggregates).
- **PRIN-02 (Small aggregates)**: Aggregates are kept as small as correctness allows (e.g., `PaymentIntent` does not embed `Invoice` — they reference each other by ID) to minimize contention and keep event streams focused.
- **PRIN-03 (UUIDv7 for all identities)**: Every aggregate, entity, and domain event uses UUIDv7 (RFC 9562) as its primary identifier. UUIDv7 is time-ordered (timestamp-prefixed), providing sequential insert performance on B-tree indexes while retaining the distributed-generation benefits of UUIDs. No UUIDv4, ULID, or other ID formats are used. All ID generation uses the `uuid_v7()` function (Rust: `uuid::Uuid::now_v7()`, Go: `github.com/google/uuid.New()`). This is enforced at the type-system level — aggregate ID types are `Uuid` (Rust) / `uuid.UUID` (Go) with no ID generation in application code outside the designated factory functions.
- **PRIN-04 (Money is never a float)**: All `Money` value objects use integer minor-unit representation; currency conversion, where it appears at all (BIZ-016), is always an explicit, recorded operation producing a new `Money` value with provenance (rate, source, timestamp), never an implicit cast.
- **PRIN-05 (Events are the audit log; there is no separate bolt-on audit table for event-sourced contexts)**: For BC-05, BC-09, BC-10, BC-08, the event stream *is* the audit trail (BIZ-040). For non-event-sourced supporting contexts (BC-01, BC-02, BC-13, BC-14), a lighter-weight append-only audit log table captures command execution (actor, timestamp, before/after) without full event sourcing overhead, since replay/rebuild-from-events is not a requirement for those contexts.

---

## 7. CQRS — Command & Query Separation Approach

- **Command side**: Every mutating operation across all contexts is modeled as an explicit `Command` type (e.g., `AuthorizePaymentCommand`, `ActivateRoutingPolicyCommand`) validated against current aggregate state before producing events. Commands are the only way to change state — there are no direct field-level updates anywhere in the domain layer.
- **Query side**: Read models are purpose-built projections (e.g., the unified transaction dashboard, Part 2 UC-070, is a ClickHouse projection; the reconciliation exception queue, UC-041, is a Postgres projection optimized for exception-list filtering) — never served by replaying the full event stream on every read.
- **Consistency expectations set here for later parts**: Command-side (aggregate) consistency is always strong/immediate. Query-side projections are eventually consistent, with a target propagation lag defined in Part 11 (NFRs) — this must be communicated in the UI (e.g., "as of" timestamps on dashboards) rather than presented as instantaneous truth.

---

## 8. Anti-Corruption Layers — Explicit List

| ACL | Protects | External System | Detailed In |
|---|---|---|---|
| Acquirer Connector ACL | BC-05's domain model | Each connected acquirer/PSP | Part 7 |
| KYB Partner ACL | BC-03's domain model | External KYB/AML decisioning API | Part 8 |
| Bank Settlement File ACL | BC-09's domain model | Bank/acquirer settlement file formats (varied) | Part 9 |

---

## 9. Missing Design Patterns — Gap Analysis Additions

### 9.1 Saga / Process Manager Pattern

The SRS describes event-driven cross-service coordination but never explicitly defines a Saga or Process Manager pattern. For a payment orchestration system, sagas are essential.

**Add the following bounded context/aggregates:**

#### BC-17 — Saga Coordinator (Cross-Cutting Infrastructure)

**Purpose**: Orchestrate multi-step, cross-aggregate business processes that span multiple bounded contexts and require compensation logic on partial failure.

**Key Sagas:**

| Saga ID | Name | Trigger | Steps | Compensation |
|---|---|---|---|---|
| SAGA-01 | Payment Lifecycle Saga | `CreatePaymentIntent` | Authorize → Capture → Settle → Reconcile | Void on capture failure; reconcile against partial state |
| SAGA-02 | Subscription Renewal Saga | Scheduler (JOB-01) | Create renewal intent → Authorize → Handle dunning on failure | Cancel subscription on exhausted retries |
| SAGA-03 | Reconciliation Resolution Saga | `ResolveReconciliationException` | Match → Confirm → Update settlement status | Undo match on confirmation failure |
| SAGA-04 | Invoice Payment Saga | `InvoiceSent` + payment completion | Create intent → Authorize → Capture → Update invoice | Void intent if invoice cancelled mid-flow |

**Implementation Rule (SAGA-001)**: Each saga is modeled as a durable state machine persisted in its own Postgres `saga_instances` table, keyed by `saga_id`. Saga state transitions are recorded as events in a dedicated `saga_events` stream, providing audit trails consistent with PRIN-05. No saga relies on in-memory state — crash recovery replays the saga event stream to rebuild current state.

**Saga Instance Entity (SeaORM — Rust):**

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "saga_instances")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub saga_id: Uuid,
    pub saga_type: String,
    pub aggregate_id: Uuid,
    pub status: String,       // 'running' | 'completed' | 'compensating' | 'failed'
    pub current_step: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

**Saga Instance Entity (Ent — Go):**

```go
// schema/saga_instance.go
package schema

import (
    "entgo.io/ent"
    "entgo.io/ent/schema/field"
)

type SagaInstance struct {
    ent.Schema
}

func (SagaInstance) Fields() []ent.Field {
    return []ent.Field{
        field.UUID("id", uuid.UUID{}).Immutable(),
        field.String("saga_type"),
        field.UUID("aggregate_id", uuid.UUID{}),
        field.Enum("status").Values("running", "completed", "compensating", "failed"),
        field.String("current_step"),
        field.Time("created_at").Default(time.Now).Immutable(),
        field.Time("updated_at").Default(time.Now).UpdateDefault(time.Now),
    }
}
```

**Design Principle (SAGA-002)**: Sagas never hold custody of funds (consistent with Part 1 §6). A saga's compensation logic for payment flows is always "revert the orchestration state machine" (void, cancel), never "move funds back through a platform-controlled account."

**Design Principle (SAGA-003)**: Saga steps must be idempotent — compensation actions (void, cancel) must detect and skip already-completed operations rather than failing on duplicate execution. This is enforced by checking the target aggregate's current state before executing the compensation action.

**Design Principle (SAGA-004)**: Saga steps have configurable timeouts (default: 30 seconds for synchronous steps, 5 minutes for async steps). When a step times out, the saga transitions to `Compensating` state and triggers compensation for all completed steps. Timeout values are part of the saga configuration, not hardcoded.

**Design Principle (SAGA-005)**: Concurrent sagas operating on the same aggregate are detected via optimistic concurrency control (Part 5 CONC-001) — if two sagas attempt to mutate the same PaymentIntent, one will fail the concurrency check and must retry after reloading the aggregate state.

### 9.2 Outbox Pattern (Transactional Outbox)

Event publishing reliability requires the Transactional Outbox pattern to guarantee that domain events are published to NATS JetStream if and only if the corresponding aggregate state change commits to Postgres.

**Outbox Entity (SeaORM — Rust):**

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "outbox")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub outbox_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub payload: Vec<u8>,       // protobuf-encoded EventEnvelope
    pub created_at: DateTimeWithTimeZone,
    pub published_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

**Relay Process (OUTBOX-001)**: A dedicated relay process (implemented within each event-sourced service, not a separate microservice for MVP) polls unpublished outbox entries, publishes to NATS JetStream, and marks them published. The relay runs at sub-second polling intervals to minimize propagation lag. On crash recovery, the relay resumes from the last unconfirmed publish — at-least-once delivery is guaranteed; exactly-once effect is achieved at the consumer level per Part 4 §4.2.

**Why this matters for BIZ-040/PRIN-05**: Without the outbox pattern, a crash between Postgres commit and NATS publish would silently lose domain events, breaking the "event stream IS the audit log" claim. The outbox table is the durable bridge.

### 9.3 Circuit Breaker Pattern

**CB-001 (Acquirer Circuit Breaker)**: `connector-gateway` maintains per-connector circuit breakers. When a connector's error rate exceeds a configurable threshold (e.g., >50% error rate over a 30-second window), the circuit opens and `orchestration-service` routing logic automatically skips that connector for the duration of the open window, avoiding cascading latency degradation on the checkout path.

**CB-002 (Inter-Service Circuit Breaker)**: For non-checkout-critical synchronous calls (e.g., `compliance-service` → `document-service` for OCR), circuit breakers prevent a degraded dependency from consuming connection pool resources. On circuit open, the call fails fast with a degraded-mode response rather than queueing indefinitely.

**CB-003 (Bulkhead per Connector)**: Each acquirer adapter within `connector-gateway` has its own connection pool and timeout budget, isolated from other adapters. A hanging TCP connection to one acquirer cannot starve connection pool resources for other acquirers.

### 9.4 Retry with Exponential Backoff + Jitter

**RETRY-001**: All external-system calls (acquirer APIs, webhook delivery to merchants, settlement file polling) use exponential backoff with jitter:
- Initial delay: configurable per integration (acquirer: 100ms, webhook delivery: 1s, settlement poll: 5min)
- Multiplier: 2x per retry
- Max delay: configurable cap (acquirer: 5s, webhook: 1h, settlement poll: 1h)
- Jitter: ±25% randomization to prevent thundering herd
- Max retries: configured per integration type

**RETRY-002**: Internal service-to-service gRPC calls use a lighter retry policy (1 retry, fixed 100ms delay, only on UNAVAILABLE/DEADLINE_EXCEEDED — never on INVALID_ARGUMENT or PERMISSION_DENIED).

### 9.5 Soft-Delete and Archival Strategy

**ARCH-001**: Aggregates in terminal states (`PaymentIntent` in `Captured`/`Refunded`/`Voided`/`Failed`/`AuthorizationExpired`, `Invoice` in `Paid`/`Cancelled`, `Subscription` in `Cancelled`) are candidates for archival after a configurable retention period (default: 90 days in hot store). Archival moves the aggregate's event stream from `event_store` to a cold-storage table (`event_store_archive`) with the same schema but on a separate tablespace.

**ARCH-002**: Snapshotting frequency (Part 9 DB-003) is adjusted for archived aggregates — no new snapshots are created after archival.

**ARCH-003**: Read-model projections for archived aggregates are maintained indefinitely, but the underlying event streams are only rehydrated on demand for compliance/audit purposes.

### 9.6 Tenant Provisioning Orchestration

**PROV-001**: Tenant onboarding (UC-001) triggers a provisioning saga that creates all required per-tenant infrastructure: Postgres schema/role, MinIO bucket, OpenSearch index, Redis key prefix namespace, NATS stream consumer configuration.

**PROV-002**: Provisioning is idempotent — re-running the provisioning saga for an already-provisioned tenant is a no-op (checked via `tenant.provisioned_at` timestamp).

**PROV-003**: Tenant suspension disables live processing but retains all data for audit compliance (AUD-001). Deprovisioning (data deletion) is deferred pending OQ-019 legal confirmation.

### 9.7 API Key Scoping to Acquirer Links

**APIKEY-001**: In addition to role-based permission scoping (Part 8 §2.1), API keys can optionally be scoped to specific `MerchantAcquirerLink` IDs, so that a merchant integration for a specific acquirer can only interact with that acquirer's data, following the principle of least privilege.

### 9.8 Global Tenant Event Ordering

**EVT-ORDER-001**: For use cases requiring cross-aggregate chronological ordering (AI Assistant summary documents, analytics dashboards), a per-tenant global event sequence is assigned by a lightweight `tenant_event_counter` table incremented atomically alongside event store appends:

**Event Counter Entity (SeaORM — Rust):**

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "event_counter")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,               // always 1 (singleton row)
    pub next_sequence: i64,
}
```

This counter is NOT used for aggregate consistency (that's `event_sequence`); it's purely a read-model concern for cross-aggregate ordering.

---

## 10. Traceability to Part 1 / Part 2

| Part 1/2 Requirement | Enforced By (this Part) |
|---|---|
| BIZ-010 (configurable routing, no code change) | BC-05 `RoutingPolicy` aggregate, versioned rules (INV-05) |
| BIZ-011 (no custody) | Structural absence of any "platform-owned balance" aggregate anywhere in this catalog |
| BIZ-012 (failover) | AGG-01 `RoutingAttempt` entity + EVT-02/EVT-06/EVT-07 |
| BIZ-013 (unified ledger/reconciliation) | BC-09 `SettlementBatch`/`SettlementRecord` |
| BIZ-020/021/023 (AI grounded, citable, self-hosted) | BC-12 modeled as read-only Conformist with `GroundingCitation` records |
| BIZ-040 (immutable audit) | Event sourcing discipline (PRIN-05), full envelope in §4 |
| BR-020-1/INV-02 (no double capture/auth) | AGG-01 invariants INV-01/INV-02 |
| BR-022-1/INV-03 (refund same acquirer) | AGG-01 invariant INV-03 |
| EX-080a (no fallback to platform custody on split failure) | AGG-05 invariant INV-09 |
| Saga/Process Manager (cross-aggregate workflows) | BC-17 Saga Coordinator, SAGA-001 through SAGA-002 |
| Transactional Outbox (event publish reliability) | §9.2 OUTBOX-001, outbox table |
| Circuit Breaker / Bulkhead (acquirer resilience) | §9.3 CB-001 through CB-003 |
| Retry backoff+jitter (external system calls) | §9.4 RETRY-001, RETRY-002 |
| Soft-delete/archival (event store lifecycle) | §9.5 ARCH-001 through ARCH-003 |
| Tenant provisioning orchestration | §9.6 PROV-001 through PROV-003 |
| API key scoping (least privilege) | §9.7 APIKEY-001 |
| Cross-aggregate event ordering | §9.8 EVT-ORDER-001 |

---

## 11. Open Items Carried Forward

- **OQ-007**: Confirm whether `RiskAssessment` (BC-11) should be its own bounded context or a value object embedded in `PaymentIntent` once ML-based scoring (H3) is designed in detail — kept separate for now to avoid coupling BC-05's release cadence to fraud-model iteration speed, but should be revisited in Part 5.
- **OQ-008**: Confirm event retention/replay policy in NATS JetStream (how long raw event streams are retained vs. archived to object storage) — affects whether "rebuild aggregate from full event history" remains cheap indefinitely or requires snapshotting; addressed in Part 4/9.
- **OQ-029**: Finalize saga persistence strategy — whether saga state is stored in the same Postgres database as the aggregate it orchestrates or in a dedicated saga database. Recommended: same database for MVP (simpler transactional guarantees), split later if saga volume warrants it.
- **OQ-030**: Determine outbox relay polling interval trade-offs — sub-second polling adds Postgres load; consider CDC via Debezium for production scale. Decision deferred to Part 11 capacity planning.
- **OQ-031**: Finalize circuit breaker thresholds (CB-001 error-rate threshold, open-window duration) against real acquirer failure-mode data from pilot merchants.
- **OQ-032**: Confirm archival retention period (ARCH-001 default 90 days) against legal/compliance retention floor (Part 8 AUD-001, OQ-018) — archival must not move data out of reach before the retention floor expires.

---

*End of Part 3. Proceed to Part 4: Microservice Architecture.*
