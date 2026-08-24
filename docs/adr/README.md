# Architecture Decision Records (ADRs)

> This directory contains Architecture Decision Records for the Payment Orchestra platform. ADRs document significant architectural decisions with their context, rationale, consequences, and current status.

**Total Records:** 14  
**Status Legend:** ✅ Accepted | 🔄 Superseded | ❌ Rejected | ⏳ Proposed

---

## Quick Reference

| ADR | Title | Status | Date |
|---|---|---|---|
| [ADR-001](#adr-001-modular-monolith-over-microservices) | Modular Monolith over Microservices | ✅ Accepted | 2026-07-22 |
| [ADR-002](#adr-002-rest-paths--protobuf-bodies-over-pure-grpc-or-rest-json) | REST Paths + Protobuf Bodies | ✅ Accepted | 2026-07-22 |
| [ADR-003](#adr-003-postpatchdelete-only-no-get) | POST/PATCH/DELETE Only (No GET) | ✅ Accepted | 2026-07-22 |
| [ADR-004](#adr-004-in-process-nats-over-external-nats-server) | In-Process NATS over External NATS Server | ✅ Accepted | 2026-07-22 |
| [ADR-005](#adr-005-postgresql-event-store-over-specialized-event-stores) | PostgreSQL Event Store over Specialized Event Stores | ✅ Accepted | 2026-07-22 |
| [ADR-006](#adr-006-event-sourcing-for-core-services-only) | Event Sourcing for Core Services Only | ✅ Accepted | 2026-07-22 |
| [ADR-007](#adr-007-byok-bring-your-own-key-over-payment-facilitation) | BYOK over Payment Facilitation | ✅ Accepted | 2026-07-22 |
| [ADR-008](#adr-008-rust-with-seaorm-over-gojava) | Rust with SeaORM over Go/Java | ✅ Accepted | 2026-07-22 |
| [ADR-009](#adr-009-single-module-internal-structure-convention) | Single Module Internal Structure Convention | ✅ Accepted | 2026-07-22 |
| [ADR-010](#adr-010-in-process-grpc-for-synchronous-inter-module-calls) | In-Process gRPC for Synchronous Inter-Module Calls | ✅ Accepted | 2026-07-22 |
| [ADR-011](#adr-011-transactional-outbox-for-event-publish-reliability) | Transactional Outbox for Event Publish Reliability | ✅ Accepted | 2026-07-22 |
| [ADR-012](#adr-012-circuit-breaker-per-merchant-acquirer-link) | Circuit Breaker per Merchant-Acquirer Link | ✅ Accepted | 2026-07-22 |
| [ADR-013](#adr-013-abac-attribute-based-access-control-over-rbac) | ABAC over RBAC | ✅ Accepted | 2026-07-22 |
| [ADR-014](#adr-014-envelope-encryption-for-credential-storage) | Envelope Encryption for Credential Storage | ✅ Accepted | 2026-07-22 |

---

## ADR-001: Modular Monolith over Microservices

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Consulted** | Engineering Team |
| **Last Modified** | 2026-07-22 |

### Context

The platform needs to serve as a payment orchestration layer with clear domain boundaries (operator management, IAM, payment orchestration, reconciliation, etc.). The initial architecture question was whether to deploy these as separate microservices or as a modular monolith.

**Key considerations:**

- **Team size**: Single engineering team at launch — microservices overhead (CI/CD pipelines per service, deployment coordination, service mesh configuration, inter-service auth) would consume disproportionate engineering capacity.
- **Latency sensitivity**: The checkout hot path (API Gateway → Orchestration → Connector → Acquirer) has a strict 10-second total deadline. In-process calls eliminate network latency and serialization overhead.
- **Module boundaries**: The bounded contexts from the DDD exercise (Part 3) are well-defined. The question is whether to enforce them at the process level (microservices) or the code level (monolith modules).
- **Future extraction**: If traffic patterns demand it, any module can be extracted into a separate microservice because all inter-module communication goes through proto-defined interfaces.

### Decision

Adopt a **modular monolith** architecture:

- All domain modules compile into a single binary
- Module boundaries are enforced at the code level (separate Rust crates within a workspace)
- Inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous)
- All modules share a single PostgreSQL instance with per-module schema isolation
- Horizontal scaling is achieved by running multiple replicas behind a load balancer

### Consequences

**Positive:**

- 🟢 Simplified deployment — one Docker image, one CI pipeline
- 🟢 Zero network latency for inter-module calls
- 🟢 No service mesh, no mTLS between modules, no distributed tracing complexity for internal calls
- 🟢 Faster development velocity — no need to coordinate cross-service deployments for local testing
- 🟢 Single binary to monitor, debug, and operate

**Negative:**

- 🔴 All modules scale together — cannot independently scale the orchestration module independently from analytics
- 🔴 A crash in any module takes down all modules (mitigated by Rust's memory safety + process isolation via separate OS threads/tasks)
- 🔴 Build time scales with total codebase size
- 🔴 Module extraction requires decomposing the monolith later, which is technically feasible but requires discipline

**Mitigations:**

- Module interfaces are defined exclusively through protobuf contracts — extraction is a deployment topology change, not a code rewrite
- Resource-heavy modules (AI Assistant) communicate via in-process calls but run their inference on external GPU nodes

### Related

- Supersedes: N/A (first architecture decision)
- Related decisions: ADR-004 (in-process NATS), ADR-010 (in-process gRPC)
- References: SRS Part 04 §8 (Deployment Topology)

---

## ADR-002: REST Paths + Protobuf Bodies over Pure gRPC or REST JSON

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

The platform exposes an external API for merchants to integrate with. Industry options include:

1. **Pure gRPC**: Strongly typed, performant, but requires gRPC-aware clients and tooling
2. **REST JSON**: Developer-friendly (curl, browser dev tools), but runtime-typed, verbose, no schema enforcement
3. **REST paths + protobuf bodies**: RESTful URL patterns with protobuf-encoded request/response bodies
4. **GraphQL**: Flexible queries but complex caching and payment-specific mutations map poorly to GraphQL semantics

**Key considerations:**

- **Developer experience**: Merchants need to integrate quickly. RESTful URL patterns (`/v1/payment-intents/:id`) are universally understood. Pure gRPC adds friction for merchants without gRPC experience.
- **Type safety**: Payment systems cannot tolerate runtime parsing errors. Protobuf binary encoding enforces schema validation at the protocol level.
- **Single source of truth**: Both external API and internal gRPC should use the same `.proto` definitions — no duplication between a REST JSON spec and protobuf schemas.
- **SDK generation**: Protobuf definitions can generate typed SDKs for all major languages automatically.

### Decision

Use **RESTful URL paths with protobuf-encoded request/response bodies**:

- URL style: RESTful paths (`/v1/payment-intents/:id`)
- Content-Type: `application/protobuf` (binary, never JSON)
- The `.proto` file is the single source of truth for both external API and internal gRPC
- Internal service-to-service communication uses native gRPC with the same protobuf schemas

### Consequences

**Positive:**

- 🟢 Developer familiarity — RESTful URLs are intuitive and consistent
- 🟢 Type safety at wire level — protobuf enforces schema, field types, required fields
- 🟢 Automatic SDK generation from `.proto` files
- 🟢 Smaller payload size than JSON (binary protobuf)
- 🟢 Single schema to maintain for both external and internal APIs

**Negative:**

- 🔴 Protobuf binary is not human-readable in curl/browser dev tools (mitigated by `protoc --decode` and hex dump tools)
- 🔴 Less ecosystem support than JSON (no Postman native protobuf support without plugins)
- 🔴 Requires protobuf compilation step in client SDK builds

### Related

- Supersedes: N/A (first API design decision)
- Related decisions: ADR-003 (POST/PATCH/DELETE only)
- References: SRS Part 10 §1 (API Design Conventions), `17-api-gateway.md` §1

---

## ADR-003: POST/PATCH/DELETE Only (No GET)

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

HTTP GET is conventionally used for read operations with query parameters. However, this design decision explicitly removes GET from the allowed HTTP methods.

**Key considerations:**

- **Consistent request/response format**: If reads use GET with query strings and writes use protobuf bodies, clients must handle two different formats. By using POST for reads (with protobuf body), all operations have consistent format.
- **Complex filter criteria**: Payment searches often need complex nested filters (date ranges, amounts, statuses, card schemes, acquirers). URL query strings have length limitations (~2KB) and don't support nested structures well.
- **Auth/Rate-limiting consistency**: All operations can use the same authentication, rate limiting, and audit middleware when everything goes through the POST/PATCH/DELETE pipeline.
- **Idempotency**: POST operations with `Idempotency-Key` headers are idempotent — this applies to searches too (same search → same results).

### Decision

Support only **POST**, **PATCH**, and **DELETE** HTTP methods:

| Method | Purpose | Body | Idempotent |
|---|---|---|---|
| `POST` | Create, List/Search, Actions | Required (protobuf) | Yes (via `Idempotency-Key` header) |
| `PATCH` | Partial update | Required (protobuf) | Yes (via `Idempotency-Key` header) |
| `DELETE` | Delete/Disable | Optional (protobuf with reason) | Yes |
| `GET` | **NOT SUPPORTED** | — | — |

Read operations use `POST` with `/search` suffix and filter criteria in the protobuf body:
```
POST /v1/payment-intents/search
<SearchPaymentIntentsRequest protobuf>
```

### Consequences

**Positive:**

- 🟢 Consistent middleware pipeline (auth, audit, rate limiting) for all operations
- 🟢 Complex nested filters in protobuf — no URL length limitations
- 🟢 All operations are idempotent — simplifies retry logic
- 🟢 Single request format for clients to implement

**Negative:**

- 🔴 Violates REST convention (reads are not idempotent in HTTP spec — but our POST searches ARE idempotent)
- 🔴 Caching layers (CDN, browser cache) cannot cache search results (POST responses are not cacheable by default)
- 🔴 May surprise developers familiar with REST conventions
- 🔴 Tooling (curl, browser dev tools) requires explicit POST requests even for simple lookups

### Related

- Related decisions: ADR-002 (REST paths + protobuf bodies)
- References: `17-api-gateway.md` §2.2

---

## ADR-004: In-Process NATS over External NATS Server

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

The platform needs an asynchronous messaging mechanism for domain events. Options considered:

1. **External NATS JetStream cluster**: Industry-standard, durable, at-least-once delivery, DLQ support
2. **Apache Kafka**: Higher throughput but heavier operational burden for a single-team startup
3. **RabbitMQ**: AMQP-based, but less suited for at-least-once event streaming
4. **In-process channels (tokio broadcast/mpsc)**: Zero network overhead, no external dependency
5. **In-process NATS-compatible API**: In-process implementation that exposes the same API as NATS

**Key considerations:**

- **Modular monolith context**: All modules run in the same process. An external message broker adds:
  - Deployment complexity (another service to manage)
  - Network latency for what are logically in-process calls
  - TLS termination, certificate management, authentication
  - Operational cost
- **Module boundaries**: Even though communication is in-process, module boundaries must be preserved. An API that looks like NATS ensures that extracting a module later requires only changing the transport, not the code.
- **Durability**: In a monolith, if the process dies, all in-memory messages are lost. The transactional outbox pattern (ADR-011) provides durability — messages are written to PostgreSQL before being published to in-process channels.

### Decision

Use **in-process NATS-compatible channels** (tokio `broadcast` for fan-out, `mpsc` for work queues) that expose the same API surface as NATS JetStream:

```rust
// The API looks like NATS:
event_bus.publish("events.orchestration.payment_intent.payment_authorized.v1", payload).await?;
event_bus.subscribe("events.orchestration.payment_intent.*.v1").await?;
```

- Subject naming: `events.<context>.<aggregate>.<event_name>.v<version>`
- All payloads protobuf-encoded
- Outbox relay ensures durability before publishing
- DLQ implemented as PostgreSQL-backed retry queue
- Module boundaries preserved — extraction just replaces the transport layer

### Consequences

**Positive:**

- 🟢 Zero network latency — events are delivered within the same process
- 🟢 No external dependency to deploy, manage, or monitor
- 🟢 No TLS certificate management for inter-module communication
- 🟢 Module boundaries preserved via NATS-compatible API
- 🟢 Extraction path is clear — replace in-process channel with gRPC/NATS client

**Negative:**

- 🔴 Events lost if process crashes before outbox relay publishes (mitigated by transactional outbox — outbox writes in same DB transaction as aggregate state change)
- 🔴 No built-in dead letter queue (mitigated by PostgreSQL-backed DLQ)
- 🔴 Cannot independently scale subscribers (all run in the same process)
- 🔴 No built-in at-least-once delivery (must implement via outbox + idempotent projections)

### Related

- Related decisions: ADR-001 (modular monolith), ADR-011 (transactional outbox)
- References: SRS Part 04 §4.3 (NATS Subject Taxonomy), `19-infrastructure-cross-cutting.md` §1

---

## ADR-005: PostgreSQL Event Store over Specialized Event Stores

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

Event-sourced services need an event store. Options considered:

1. **PostgreSQL** (append-only table): Most familiar to the team, no additional infrastructure
2. **EventStoreDB**: Purpose-built for event sourcing, but additional operational overhead
3. **Apache Kafka**: Durable log, but higher latency and operational complexity
4. **DynamoDB/Aurora**: Managed cloud service, but vendor lock-in and cost

**Key considerations:**

- **Infrastructure diversity**: The platform already uses PostgreSQL for CRUD data. Adding another datastore increases operational burden, backup/recovery complexity, and the team's learning surface.
- **Consistency**: Event store appends happen in the same PostgreSQL transaction as aggregate state changes (via outbox pattern). This is trivially achieved with PostgreSQL — cross-datastore transactions are not.
- **Query capability**: Event sourcing often needs to replay events by aggregate ID, sequence number, or time range. PostgreSQL's query capabilities (indexes, pagination, window functions) handle this well.
- **Performance**: PostgreSQL can handle 1000+ writes/second on modest hardware. For a single-tenant deployment at launch, this is sufficient.

### Decision

Use **PostgreSQL as the event store** with:

- Append-only `event_store` table with composite primary key on `(aggregate_type, aggregate_id, event_sequence)`
- Optimistic concurrency via `expected_event_sequence` on append
- Protobuf-encoded payloads (not JSON)
- Periodic snapshotting for long-lived aggregates (via `aggregate_snapshot` table)
- Event archival after retention period (via `event_store_archive` table)

### Consequences

**Positive:**

- 🟢 No new infrastructure — uses existing PostgreSQL deployment
- 🟢 Strong consistency — event store append and outbox write in the same transaction
- 🟢 Full SQL query capability for event replay and debugging
- 🟢 Familiar tooling, backup, and recovery procedures
- 🟢 Row-Level Security (RLS) for defense-in-depth

**Negative:**

- 🔴 PostgreSQL is not a log-structured store — sequential writes to event_store create index fragmentation (mitigated by periodic `VACUUM` reindexing, or using `pg_partman` for time-based partitioning)

### Related

- Related decisions: ADR-006 (event sourcing for core services only), ADR-011 (transactional outbox)
- References: SRS Part 09 §1 (Event Store Design)

---

## ADR-006: Event Sourcing for Core Services Only

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

Event sourcing adds complexity (event store, projections, replay, snapshotting) but provides a complete audit trail and temporal query capability. The question is which services justify this complexity.

**Services requiring event sourcing:**

| Service | Rationale |
|---|---|
| `orchestration-service` | **Core** — financial transactions (authorizations, captures, refunds) require immutable audit trail. State machine has complex transitions. | 
| `reconciliation-service` | **Core** — settlement matching needs to track what was expected vs. what arrived. Temporal queries needed for exception resolution. |
| `subscription-service` | Changes over time (price changes, plan changes, proration events). Must be able to explain every billing decision. |
| `invoice-service` | Financial records with legal retention requirements. Revised from CRUD to event-sourced for audit trail. |

**Services NOT requiring event sourcing:**

| Service | Reason |
|---|---|
| `operator-service` | Simple CRUD — operator lifecycle is straightforward. Audit logged via `audit_log` table. |
| `iam-service` | Permission changes are already audit-logged. No need for full event replay. |
| `compliance-service` | KYB case status changes are simple state machines. Current state is sufficient. |
| `connector-gateway` | Stateless protocol translators. No aggregate state to event-source. |
| `payment-link-service` | Read-heavy/public-facing. CRUD + events is sufficient. |
| `dispute-service` | Dispute cases have simple state machines. Events are published for notification. |
| `risk-service` | Rules-based scoring — no aggregate state to event-source. |
| Others | All remaining services have simple data models that don't benefit from event sourcing. |

### Decision

Use **event sourcing** for exactly four services: **orchestration**, **reconciliation**, **subscriptions**, and **invoices**.

All other services use **CRUD + events**: standard CRUD operations with domain events published for downstream consumers.

### Consequences

**Positive:**

- 🟢 Full immutable audit trail for financial transactions
- 🟢 Temporal queries for reconciliation and billing disputes
- 🟢 Avoids unnecessary complexity in services where current state is sufficient
- 🟢 Reduced event store storage for non-essential services

**Negative:**

- 🔴 Inconsistency: Some services support temporal queries, others don't. Team must remember which is which.
- 🔴 If a non-event-sourced service later needs event sourcing (e.g., risk-service for ML training), migration is expensive

### Related

- Related decisions: ADR-005 (PostgreSQL event store)
- References: `docs/backend/README.md` (Service Index)

---

## ADR-007: BYOK (Bring Your Own Key) over Payment Facilitation

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Consulted** | Product, Legal, Compliance |
| **Last Modified** | 2026-07-22 |

### Context

The platform can operate in one of two models:

1. **Payment Facilitation (PayFac)**: The platform becomes a payment facilitator, onboarding merchants under its own merchant account. Merchants don't need their own gateway accounts.
2. **BYOK (Bring Your Own Key)**: Merchants connect their own existing gateway credentials. The platform routes payments through the merchant's own gateway accounts.

**Key considerations:**

- **Compliance burden**: Payment facilitation requires PCI DSS Level 1 compliance, holds the platform responsible for merchant underwriting, and creates liability for chargebacks and fraud.
- **Regulatory landscape**: UAE Central Bank regulations favor the BYOK model for payment orchestration platforms. Multiple acquirer relationships per merchant are common.
- **Merchant needs**: Enterprise merchants in the UAE already have established relationships with acquirers (Checkout.com, Adyen, Moyasar, etc.). They want a single integration point to manage multiple acquirers, not another payment gateway.
- **Differentiation**: The market needs a routing layer, not another payment gateway. BYOK enables value-added services (smart routing, failover, reconciliation, analytics) on top of existing merchant accounts.
- **Single-tenant, no-custody**: The platform is a pure routing layer — it never holds, touches, or controls funds. Money flows directly between the customer, the merchant's payment gateway, and the merchant's bank account. The platform orchestrates and routes transaction *instructions* between these parties. This shapes every domain decision.

### Decision

Adopt the **BYOK (Bring Your Own Key)** model:

- The platform is a pure routing layer, NOT a payment gateway, payment facilitator, or funds holder. Money flows directly between the customer, the merchant's payment gateway, and the merchant's bank account. The platform routes transaction instructions only and never touches the actual funds.
- Merchants bring their existing gateway credentials and manage their own merchant accounts
- The `merchant-acquirer-link-service` (SVC-21) manages the credential lifecycle
- Credentials are encrypted at rest via envelope encryption (KMS-managed KEK + per-link DEK)
- Each connector defines an `OnboardingSchema` for dynamic credential forms
- Credentials are validated via sandbox/status-check before activation

### Consequences

**Positive:**

- 🟢 Reduced compliance burden — platform does not hold funds or underwrite merchants
- 🟢 Merchants keep their existing acquirer relationships and negotiated rates
- 🟢 Enables multi-acquirer routing, failover, and reconciliation
- 🟢 Clear market differentiation as an orchestration layer
- 🟢 Faster time to market — no need for payment facilitator licensing

**Negative:**

- 🔴 Merchants need their own gateway accounts — adds friction to onboarding
- 🔴 Each merchant may have different credentials, API versions, and acquirer-specific configurations
- 🔴 Credential management adds complexity (encryption, rotation, validation, expiry monitoring)
- 🔴 Circuit breaker states are per-merchant per-acquirer — more granular than a shared gateway pool

### Related

- References: `21-merchant-acquirer-link-service.md`, `22-merchant-connector-onboarding.md`, SRS Part 04 §11.3

---

## ADR-008: Rust with SeaORM over Go/Java

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

The platform's backend needs a language and ORM framework. Primary contenders:

1. **Rust + SeaORM**: Modern systems language with compile-time type safety, zero-cost abstractions, and memory safety
2. **Go + Ent/GORM**: Simple, fast compilation, large ecosystem, but weaker type system
3. **Java/Kotlin + JPA/Hibernate**: Enterprise standard, extensive ecosystem, but heavier runtime

**Key considerations:**

- **Performance**: Payment orchestration on the checkout hot path demands low latency and predictable performance. Rust's zero-cost abstractions and no-GC runtime provide this.
- **Type safety**: Payment transactions cannot tolerate type errors (currency codes, amount precision, state machines). Rust's type system catches these at compile time.
- **Memory safety**: Payment systems handle sensitive data. Rust's ownership model prevents memory safety vulnerabilities by construction.
- **ORM maturity**: SeaORM provides compile-time query checking, async support, and migration tooling. It's not as mature as Ent/GORM or JPA, but sufficient for the platform's CRUD and event store patterns.
- **Team expertise**: The team has Rust experience from prior systems programming roles.

### Decision

Use **Rust (edition 2024) with SeaORM**:

- Rust channel: stable, minimum version 1.85.0
- Edition: 2024
- SeaORM for PostgreSQL data access (event store, CRUD entities)
- Tonic for gRPC framework
- Prost for protobuf code generation
- Tokio for async runtime

### Consequences

**Positive:**

- 🟢 Compile-time memory safety — eliminates an entire class of security vulnerabilities
- 🟢 Zero-cost abstractions — no runtime overhead for type safety
- 🟢 Strong type system catches currency/amount/state errors at compile time
- 🟢 SeaORM compile-time query checking
- 🟢 Excellent async performance for concurrent request handling

**Negative:**

- 🔴 Steep learning curve for new team members
- 🔴 Slower compile times than Go (mitigated by workspace structure and incremental compilation)
- 🔴 Smaller ecosystem for payment-specific libraries
- 🔴 Protobuf integration (tonic + prost) is less mature than gRPC-Go
- 🔴 SeaORM is newer and less battle-tested than Ent/GORM

### Related

- References: `platform-backend/Cargo.toml`, `rust-toolchain.toml`

---

## ADR-009: Single Module Internal Structure Convention

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

Every service module needs internal organization. Without a consistent convention, each developer will organize code differently, making it harder to navigate between modules.

### Decision

Adopt a consistent internal structure for every module:

```
services/<service-name>/
├── Cargo.toml
└── src/
    ├── main.rs              # Entry point (or lib.rs for library crates)
    ├── config.rs            # Environment + config loading
    ├── domain/              # Aggregates, entities, value objects
    ├── commands/            # Command handlers (one file per command)
    ├── queries/             # Read-model query handlers
    ├── events/              # Event definitions + projection logic
    ├── repository/          # SeaORM data access + event store
    ├── api/                 # gRPC service implementation + health checks
    ├── pipeline/            # Command processing pipeline + middleware
    └── tests/               # Unit, integration, and property-based tests
```

### Consequences

- 🟢 Consistent navigation — any team member knows where to find code in any module
- 🟢 New modules follow the same pattern — no architectural decisions per-module
- 🔴 Overhead of maintaining the convention across PRs (CI lint checks enforce structure)

---

## ADR-010: In-Process gRPC for Synchronous Inter-Module Calls

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

Synchronous inter-module calls (e.g., Orchestration → Connector Gateway for authorization) need a communication mechanism. Options:

1. **In-process function calls** (direct Rust trait invocation): Fast but tightly coupled
2. **In-process gRPC**: Same protobuf interfaces as external API, preserves module boundaries
3. **HTTP/REST**: Network call even within same process — unnecessary overhead

### Decision

Use **in-process gRPC** (via tonic in-process channel) for all synchronous inter-module calls:

- Same protobuf service definitions as external API
- Zero-copy shared memory (no serialization/deserialization overhead)
- Deadline propagation enforced at every hop
- gRPC metadata carries actor context, correlation ID, causation ID

### Consequences

- 🟢 Same interfaces for internal and external calls — no translation layer needed
- 🟢 Zero network overhead — same-process direct invocation
- 🟢 Deadlines prevent cascading failures
- 🔴 In-process gRPC adds some overhead vs. direct trait invocation (context propagation, middleware pipeline)
- 🔴 Module extraction requires replacing in-process gRPC with network gRPC — but the interface definition is the same

### Related

- ADR-001 (modular monolith), ADR-002 (REST paths + protobuf bodies)
- References: `23-backend-architecture.md` §6.1

---

## ADR-011: Transactional Outbox for Event Publish Reliability

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

Event-sourced services need to publish events after appending them to the event store. The fundamental challenge is ensuring that events are published if and only if the event store append succeeds.

Options:

1. **Dual-write (event store + publish in application code)**: Risk of events being published even if the store append fails, or events not being published even if the append succeeds
2. **Transactional outbox**: Write event to an outbox table in the same PostgreSQL transaction as the event store append. A separate relay process reads and publishes.
3. **CDC (Change Data Capture)**: Read event store changes from PostgreSQL WAL. More complex infrastructure.

### Decision

Use the **Transactional Outbox Pattern**:

- Outbox entry is written in the same PostgreSQL transaction as the event store append
- Dedicated relay process polls unpublished entries, publishes to in-process NATS channels, marks as published
- If the relay crashes before publishing, it resumes from the last unconfirmed entry on restart
- Outbox entries are purged after 7 days

### Consequences

- 🟢 Strong guarantee: event is published iff it's committed to PostgreSQL
- 🟢 No dual-write hazards (partial failure where event is published but not stored, or stored but not published)
- 🟢 Outbox relay is a background task within the event-sourced service — no separate deployment
- 🔴 Event publishing is eventually consistent (relay poll interval adds latency)
- 🔴 Outbox table can grow if relay falls behind (monitoring and backpressure in place)

### Related

- ADR-004 (in-process NATS), ADR-005 (PostgreSQL event store)
- References: `19-infrastructure-cross-cutting.md` §1

---

## ADR-012: Circuit Breaker per Merchant-Acquirer Link

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Last Modified** | 2026-07-22 |

### Context

In a multi-acquirer routing system, an acquirer can become unhealthy (high error rate, timeout, rate-limited). The system must detect this and route traffic away from the failing acquirer.

Options:

1. **Global circuit breaker**: Single circuit breaker per connector type (e.g., one for all Checkout.com connections)
2. **Per-merchant-acquirer-link circuit breaker**: Each merchant's connection to each acquirer has its own circuit breaker
3. **Adaptive routing**: Dynamically adjust traffic distribution based on real-time success rates (Phase 3 goal)

### Decision

Implement a **circuit breaker per `MerchantAcquirerLink`**:

- States: `Closed` (normal) → `Open` (failing, traffic diverted) → `HalfOpen` (testing recovery)
- Opens when error rate exceeds 50% in a 30-second window
- Stays open for 60 seconds, then transitions to `HalfOpen`
- In `HalfOpen`, limited requests allowed (default: 3). If all succeed → `Closed`. If any fail → `Open`.
- Orchestration service skips acquirers with open circuit breaker during routing

### Consequences

- 🟢 Isolates failures — one merchant's failing acquirer doesn't affect other merchants
- 🟢 Automatic failover without operator intervention
- 🟢 Circuit breaker state feeds into routing decisions
- 🔴 More state to manage (N links = N circuit breakers)
- 🔴 Configuration per link (thresholds, timeouts) needs careful tuning
- 🔴 Circuit breaker open → all traffic to that link blocked, even for non-failing payment methods

### Related

- ADR-007 (BYOK model)
- References: `04-connector-gateway.md`, SRS Part 04 §11.2, `21-merchant-acquirer-link-service.md`

---

## ADR-013: ABAC (Attribute-Based Access Control) over RBAC

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Consulted** | Security, Compliance |
| **Last Modified** | 2026-07-22 |

### Context

The platform needs an authorization model. The primary contenders are:

1. **RBAC (Role-Based Access Control)**: Users are assigned roles, roles have permissions. Simple and widely understood, but rigid.
2. **ABAC (Attribute-Based Access Control)**: Access decisions are based on attributes of the user, resource, action, and environment. More flexible but more complex.
3. **ReBAC (Relationship-Based Access Control)**: Access based on relationships between entities (e.g., "is the owner of"). Less mature ecosystem.

**Key considerations:**

- **Fine-grained permissions**: Payment operations have complex authorization requirements. For example: "A finance operator can approve reconciliation exceptions only up to AED 10,000; above that requires an Admin." RBAC would need separate roles for each threshold, while ABAC handles this via a simple attribute condition.
- **Threshold-based approvals**: Maker/Checker pattern (Part 3 §9.2) needs amount-based conditions and role-based conditions in the same rule.
- **API key scoping**: API keys can be scoped to specific MerchantAcquirerLink IDs (APIKEY-001, Part 3 §9.8) — this is naturally expressed as an attribute condition, not a role.
- **Audit trail**: ABAC policies can be evaluated at runtime with the full decision context logged (which attributes were checked, what values they had, whether they passed). This provides richer audit data than "role X was checked."
- **Future-proofing**: As the platform grows, new access conditions (time-based, location-based, risk-score-based) can be added without role proliferation.

### Decision

Adopt **ABAC (Attribute-Based Access Control)** as the primary authorization model, with roles used as convenience groupings of common attribute sets:

- Access decisions are based on a policy defined by conditions over attributes of the principal, resource, action, and environment
- Roles exist as shorthand for common attribute bundles (e.g., "Admin" = `{department: operations, clearance_level: 5}`)
- Policies are evaluated at the command-handler level (not the API gateway) — this ensures authorization happens in the domain context where all attributes are visible
- The IAM service (SVC-02) owns policy definitions and provides the evaluation engine

```rust
// Simplified ABAC policy structure
pub struct AccessPolicy {
    pub policy_id: Uuid,
    pub action: String,                          // e.g., "resolve_reconciliation_exception"
    pub conditions: Vec<AccessCondition>,        // ALL conditions must match (AND)
    pub effect: PolicyEffect,                    // Allow | Deny
}

pub enum AccessCondition {
    Role(Vec<String>),                            // user has one of these roles
    AmountBelow { currency: String, amount: i64 },// transaction amount below threshold
    ResourceAttribute { key: String, value: String }, // resource has attribute
    TimeWindow { start: NaiveTime, end: NaiveTime }, // time-based access
    TwoPersonApproval,                            // Maker/Checker required
}
```

### Consequences

**Positive:**

- 🟢 Fine-grained, expressive policies without role explosion
- 🟢 Threshold-based conditions (amount, count) naturally supported
- 🟢 API key scoping to specific resource IDs is a simple attribute check
- 🟢 Rich audit trail — every policy evaluation context is logged
- 🟢 Extensible — new condition types can be added without changing the authorization model

**Negative:**

- 🔴 More complex than RBAC — requires policy evaluation engine and condition parsing
- 🔴 Policy evaluation latency must be minimized (sits on the critical path of every API call)
- 🔴 Policy management UI is more complex than role assignment
- 🔴 Testing requires validating combinations of attributes, not just role-permission mappings

### Related

- References: SRS Part 08 §2.2 (ABAC-001), `docs/backend/02-iam-service.md`

---

## ADR-014: Envelope Encryption for Credential Storage

| Field | Value |
|---|---|
| **Status** | ✅ Accepted |
| **Date** | 2026-07-22 |
| **Author** | Platform Architecture Team |
| **Deciders** | Platform Architecture Team |
| **Consulted** | Security, Infra |
| **Last Modified** | 2026-07-22 |

### Context

The BYOK model (ADR-007) requires the platform to store merchant gateway credentials (API keys, secrets, certificates) at rest. These are highly sensitive — a breach of credential storage would allow an attacker to make payments through the merchant's gateway accounts.

Options considered:

1. **Plaintext storage with database-level encryption**: Relies solely on Postgres TDE or volume encryption. If the database is compromised, credentials are exposed.
2. **Application-layer encryption with a single key**: All credentials encrypted with one symmetric key. If the key is compromised, all credentials are exposed. Key rotation requires re-encrypting all credentials.
3. **Envelope encryption (KMS-managed KEK + per-link DEK)**: Each `MerchantAcquirerLink` has its own Data Encryption Key (DEK), encrypted by a Key Encryption Key (KEK) stored in a KMS/Hardware Security Module (HSM).
4. **HSM-only**: All encryption operations happen inside an HSM. Strongest security but highest latency and cost.

**Key considerations:**

- **Credential isolation**: If one merchant's gateway credentials are compromised (e.g., through a side-channel attack), the damage must be contained to that one link. A per-link DEK ensures this.
- **Key rotation**: The KEK can be rotated without re-encrypting all credentials — only the DEK-wrapping operation needs to be re-executed. Per-link DEKs can be rotated independently when a merchant updates their credentials.
- **Operational complexity**: Envelope encryption adds complexity (DEK lifecycle management, caching strategy) but is a well-understood pattern (AWS KMS, GCP Cloud KMS, HashiCorp Vault all support it).
- **Compliance**: PCI-DSS and UAE Central Bank regulations require strong encryption of stored cardholder data and authentication credentials. Envelope encryption with a hardware-rooted KEK satisfies these requirements.

### Decision

Use **Envelope Encryption** for all credential storage:

- **KEK (Key Encryption Key)**: Stored in a KMS/HSM (HashiCorp Vault or cloud-native KMS). Never leaves the HSM boundary.
- **DEK (Data Encryption Key)**: One per `MerchantAcquirerLink` (per-link DEK). Generated by the KMS and returned in encrypted form (wrapped by the KEK).
- **Storage**: Encrypted DEK is stored alongside the credential record in PostgreSQL. The plaintext DEK is cached in memory (with TTL) for the lifetime of the application process.
- **Encryption algorithm**: AES-256-GCM for DEK encryption of credential payloads. The DEK itself is wrapped using the KMS's native key wrapping algorithm.
- **AAD (Additional Authenticated Data)**: Each encryption operation binds the `merchant_acquirer_link_id` as AAD, preventing ciphertext from being moved to a different record.

```rust
// Key management flow
pub struct EnvelopeEncryptionService {
    kms_client: KmsClient,
    kek_id: String,
    dek_cache: Arc<RwLock<HashMap<Uuid, Vec<u8>>>>,  // link_id -> plaintext DEK
}

impl EnvelopeEncryptionService {
    /// Encrypt credential payload for a specific link
    pub async fn encrypt_credential(
        &self,
        link_id: Uuid,
        plaintext: &[u8],
    ) -> Result<EncryptedCredential, EncryptionError> {
        let dek = self.get_or_generate_dek(link_id).await?;
        let aad = link_id.as_bytes().to_vec();
        let ciphertext = aes_256_gcm_encrypt(plaintext, &dek, &aad);
        Ok(EncryptedCredential {
            ciphertext,
            kek_version: self.current_kek_version(),
        })
    }

    /// Decrypt credential payload
    pub async fn decrypt_credential(
        &self,
        link_id: Uuid,
        encrypted: &EncryptedCredential,
    ) -> Result<Vec<u8>, EncryptionError> {
        let dek = self.get_or_generate_dek(link_id).await?;
        let aad = link_id.as_bytes().to_vec();
        aes_256_gcm_decrypt(&encrypted.ciphertext, &dek, &aad)
    }
}
```

### Consequences

**Positive:**

- 🟢 Per-link credential isolation — compromise of one DEK affects only one merchant's connection to one acquirer
- 🟢 KEK never leaves HSM — root key is hardware-protected
- 🟢 KEK rotation does not require re-encrypting credentials (only DEK wrapping)
- 🟢 AAD binding prevents ciphertext relocation attacks
- 🟢 Industry-standard pattern, well-supported by cloud KMS providers
- 🟢 Satisfies PCI-DSS and UAE regulatory encryption requirements

**Negative:**

- 🔴 KMS/HSM dependency — adds operational overhead and cost
- 🔴 DEK caching in memory — if process memory is dumped, plaintext DEKs could be exposed (mitigated by short TTL and Rust's memory safety)
- 🔴 KMS latency on DEK generation at link creation time — acceptable for a one-time operation
- 🔴 Key management complexity — KEK rotation schedule, DEK version tracking, audit logging

### Related

- Related decisions: ADR-007 (BYOK model)
- References: `21-merchant-acquirer-link-service.md`, `19-infrastructure-cross-cutting.md` §1, SRS Part 08 §3 (SEC-001)

---

## Appendix A: ADR Template

```markdown
## ADR-NNN: [Title]

| Field | Value |
|---|---|
| **Status** | ✅ Accepted / 🔄 Superseded / ❌ Rejected / ⏳ Proposed |
| **Date** | YYYY-MM-DD |
| **Author** | [Name] |
| **Deciders** | [Names or Team] |
| **Consulted** | [Names or Teams] |
| **Last Modified** | YYYY-MM-DD |

### Context

[Describe the problem, constraints, and options considered.]

### Decision

[State the decision clearly.]

### Consequences

[Positive and negative consequences.]

### Related

[Superseded by, supersedes, related ADRs, references.]
```

## Appendix B: Decision Status Flow

```
Proposed → Accepted → (may be) Superseded
         → Rejected → (may be) Reconsidered → Proposed
```

---

*End of Architecture Decision Records. Maintained by the Platform Architecture Team.*
