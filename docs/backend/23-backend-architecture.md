# 23 — Backend Architecture & Folder Structure

> **Architecture Context**: This document describes the overall backend architecture of the platform — a modular monolith with domain boundaries, organized as a Rust workspace with clearly defined module interfaces via protobuf contracts.

---

## 1. Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────┐
│                  External Clients (SDK, Dashboard)           │
└──────────────────┬──────────────────────────────────────────┘
                   │ POST/PATCH/DELETE (protobuf bodies)
                   ▼
┌─────────────────────────────────────────────────────────────┐
│                    API Gateway (api-gateway)                  │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────────────┐  │
│  │TLS Term. │  │Auth/Authz│  │ Rate Limit + CORS + Route  │  │
│  └──────────┘  └──────────┘  └───────────────────────────┘  │
└──────────────────────────┬──────────────────────────────────┘
                           │ In-process gRPC
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  MODULAR MONOLITH PROCESS                     │
│                                                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ operator │ │   iam    │ │compliance│ │connector │        │
│  │ service  │ │ service  │ │ service  │ │ gateway  │        │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘        │
│       │            │            │            │               │
│  ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐        │
│  │orchestrat.│ │ invoice  │ │ payment  │ │subscript.│        │
│  │ service   │ │ service  │ │ link svc │ │ service  │        │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘        │
│       │            │            │            │               │
│  ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐        │
│  │reconcili. │ │ dispute  │ │  risk    │ │ merchant │        │
│  │ service   │ │ service  │ │ service  │ │acq.link  │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
│                                                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │   AI     │ │ document │ │ notific. │ │analytics │        │
│  │assistant │ │ service  │ │ service  │ │ service  │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
│                                                               │
│  ┌───────────────────────────────────────────────────┐       │
│  │          Cross-Cutting Infrastructure              │       │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │       │
│  │  │ Outbox   │ │ Leader   │ │ Feature Flags    │  │       │
│  │  │ Relay    │ │ Election │ │ + Health Checks   │  │       │
│  │  └──────────┘ └──────────┘ └──────────────────┘  │       │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │       │
│  │  │ Envelope │ │Structured│ │ Degraded Mode    │  │       │
│  │  │Encryption│ │ Logging  │ │ Handlers         │  │       │
│  │  └──────────┘ └──────────┘ └──────────────────┘  │       │
│  └───────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Data Stores                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │PostgreSQL │ │  Redis   │ │ MinIO    │ │ Ollama   │        │
│  │(primary)  │ │ (cache)  │ │ (blobs)  │ │ (AI inf) │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────────────────┐ │
│  │ ClickHouse│ │OpenSearch│ │  PostgreSQL (event stores)   │ │
│  │(analytics)│ │ (vector) │ │  4 event-sourced services    │ │
│  └──────────┘ └──────────┘ └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Directory Structure

The backend lives under `platform-backend/` and is organized as a Rust workspace.

```
platform-backend/
├── Cargo.toml                          # Workspace root — defines all members
├── Cargo.lock                          # Locked dependency versions
├── rust-toolchain.toml                 # Pinned Rust version + targets
├── deny.toml                           # cargo-deny policy (supply chain security)
├── audit.toml                          # cargo-audit configuration
├── .dockerignore
├── Dockerfile                          # Multi-stage Docker build
├── docker-compose.yml                  # Local development environment
│
├── proto/                              # Protobuf service definitions
│   ├── common.proto                    #   Shared types: Money, Pagination, ErrorDetail
│   ├── operator.proto                  #   Operator service contract
│   ├── iam.proto                       #   Identity & Access Management
│   ├── compliance.proto                #   KYB/AML compliance
│   ├── connector.proto                 #   Connector gateway
│   ├── orchestration.proto             #   Payment orchestration
│   ├── invoice.proto                   #   Invoice service
│   ├── subscription.proto              #   Subscription billing
│   ├── payment-link.proto              #   Payment link (future)
│   ├── reconciliation.proto            #   Settlement reconciliation
│   ├── dispute.proto                   #   Dispute management
│   ├── risk.proto                      #   Risk scoring
│   ├── ai_assistant.proto              #   AI assistant
│   ├── document.proto                  #   Document management
│   ├── notification.proto              #   Notification + webhook delivery
│   ├── analytics.proto                 #   Analytics queries
│   ├── saga.proto                      #   Saga coordinator
│   └── orchestration.proto             #   Payment orchestration (detailed)
│
├── migrations/                         # Database migrations (SeaORM)
│   ├── Cargo.toml                      #   Migration crate manifest
│   ├── lib.rs                          #   Migration registry
│   └── src/
│       ├── m20240101_000001_create_operators.rs
│       ├── m20240101_000005_create_kyb_cases.rs
│       ├── m20240101_000009_create_event_store.rs
│       ├── m20240101_000010_create_payment_intents.rs
│       ├── m20240101_000012_create_gateway_profiles.rs
│       ├── m20240101_000013_create_invoices.rs
│       ├── m20240101_000014_create_payment_links.rs
│       ├── m20240101_000017_create_notifications.rs
│       ├── m20240101_000021_create_ai_requests.rs
│       └── m20240101_000022_create_route_configs.rs
│
├── services/                           # Domain modules (the monolith's modules)
│   ├── operator-service/               #   BC-01: Operator Management
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs                #   Entry point (or lib.rs for library crate)
│   │   │   ├── domain/                #   Aggregates, entities, value objects
│   │   │   ├── commands/              #   Command handlers
│   │   │   ├── queries/               #   Read-model query handlers
│   │   │   ├── events/                #   Event definitions + handlers
│   │   │   ├── repository/            #   SeaORM data access
│   │   │   ├── api/                   #   gRPC service implementation
│   │   │   └── tests/                 #   Unit + integration tests
│   │   └── ...
│   ├── iam-service/                   #   BC-02: Identity & Access
│   ├── compliance-service/            #   BC-03: Compliance (KYB/AML)
│   ├── connector-gateway/             #   BC-04: Connector Framework
│   ├── orchestration-service/         #   BC-05: Payment Orchestration (CORE)
│   ├── invoice-service/               #   BC-06: Invoice Service
│   ├── payment-link-service/          #   BC-07: Payment Links
│   ├── subscription-service/          #   BC-08: Subscription Billing
│   ├── reconciliation-service/        #   BC-09: Settlement & Reconciliation
│   ├── dispute-service/               #   BC-10: Dispute Management
│   ├── risk-service/                  #   BC-11: Fraud & Risk
│   ├── ai-assistant-service/          #   BC-12: AI Payment Assistant
│   ├── document-service/              #   BC-13: Document Management
│   ├── notification-service/          #   BC-14: Notifications + Webhooks
│   ├── analytics-service/             #   BC-15: Analytics & Reporting
│   ├── saga-coordinator/              #   BC-17: Saga Coordinator (library crate, compiled into orchestration)
│   ├── scheduler/                     #   Cross-cutting: Background job scheduling (library crate)
│   ├── merchant-acquirer-link-service/ # NEW BC: BYOK Core
│   ├── outbox-relay/                  #   Cross-cutting: Outbox → channel relay (background task)
│   ├── api-gateway/                   #   Cross-cutting: External API ingress
│   └── ai-gateway/                    #   Cross-cutting: AI guardrails (middleware within api-gateway)
│
├── tests/                             # Integration + security tests
│   ├── common/
│   │   └── mod.rs                     #   Shared test utilities
│   ├── operator_service_tests.rs
│   ├── iam_service_tests.rs
│   ├── connector_gateway_tests.rs
│   ├── middleware_tests.rs
│   ├── production_readiness_tests.rs
│   └── security_tests.rs
│
└── load-tests/                        # K6-based load testing
    ├── README.md
    ├── checkout.js                    #   Checkout flow load test
    └── failover.js                    #   Failover scenario load test
```

---

## 3. Module Organization Convention

### 3.1 Per-Service Internal Structure

Every service crate follows a consistent internal layout:

```
services/<service-name>/
├── Cargo.toml
└── src/
    ├── main.rs                 # Binary entry point (or lib.rs for library crates)
    ├── config.rs               # Environment + config loading
    ├── domain/
    │   ├── mod.rs
    │   ├── aggregate.rs        # Aggregate root (for event-sourced services)
    │   ├── entity.rs           # SeaORM entity definitions
    │   └── value_object.rs     # Domain value objects
    ├── commands/
    │   ├── mod.rs
    │   └── <command_name>.rs   # One file per command
    ├── queries/
    │   ├── mod.rs
    │   └── <query_name>.rs     # Read-model query handlers
    ├── events/
    │   ├── mod.rs
    │   ├── handler.rs          # Event subscription + processing
    │   └── <event_type>.rs     # Event schema + projection logic
    ├── repository/
    │   ├── mod.rs
    │   ├── event_store.rs      # Event store operations (event-sourced only)
    │   ├── outbox.rs           # Outbox writes + relay
    │   └── projection.rs       # Read-model projections
    ├── api/
    │   ├── mod.rs
    │   ├── grpc.rs             # gRPC service implementation
    │   └── health.rs           # Health check endpoints
    ├── pipeline/
    │   ├── mod.rs
    │   ├── command_handler.rs  # Command processing pipeline
    │   └── middleware.rs       # Authz, logging, metrics middleware
    └── tests/
        ├── mod.rs
        ├── unit.rs             # Unit tests per module
        ├── integration.rs      # Integration tests (testcontainers)
        └── property.rs         # Property-based tests (proptest)
```

### 3.2 Shared Crates

Cross-cutting functionality is extracted into shared crates:

| Crate | Location | Purpose |
|---|---|---|
| `platform-proto` | `proto/` | Compiled protobuf types + gRPC client/server stubs |
| `platform-logging` | `services/shared/logging/` | Structured JSON logging (LOG-SCHEMA-002) |
| `platform-kms` | `services/shared/kms/` | Envelope encryption client (SEC-001) |
| `platform-event-store` | `services/shared/event-store/` | Event store abstractions + SeaORM entities |
| `platform-outbox` | `services/shared/outbox/` | Outbox pattern + relay implementation |
| `platform-health` | `services/shared/health/` | Health check framework (HEALTH-001–004) |
| `platform-metrics` | `services/shared/metrics/` | Prometheus metrics + RED metrics |

---

## 4. Module Dependency Graph

```
                        ┌──────────────────┐
                        │  api-gateway     │ (external ingress only)
                        └────────┬─────────┘
                                 │ depends on
                                 ▼
┌─────────────────────────────────────────────────────────────┐
│                    SHARED CRATES                              │
│  platform-proto  platform-logging  platform-kms              │
│  platform-event-store  platform-outbox  platform-health       │
│  platform-metrics                                              │
└─────────────────────────────────────────────────────────────┘
                                 ▲
                                 │ used by all
                                 │
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│ iam      │  │ operator │  │compliance│  │ saga     │
│ service  │  │ service  │  │ service  │  │coordinator│
└────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
     │              │              │              │
     └──────────────┼──────────────┼──────────────┘
                    │              │
         ┌──────────▼──────────────▼──────────┐
         │        orchestration-service        │
         │  (central — most services depend)   │
         └──────────┬──────────────┬──────────┘
                    │              │
         ┌──────────▼────┐  ┌──────▼──────────┐
         │ connector     │  │  risk-service   │
         │ gateway       │  │  (scoring)      │
         └───────┬───────┘  └─────────────────┘
                 │
         ┌───────▼────────────────────┐
         │ merchant-acquirer-link     │
         │ service (BYOK)             │
         └───────┬────────────────────┘
                 │
                 ▼
         ┌────────────────────┐
         │ Acquirer Adapters  │
         │ (in-process plugins) │
         │ Checkout, Stripe,  │
         │ Adyen, Moyasar ... │
         └────────────────────┘

┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│ invoice  │  │payment   │  │subscript.│  │reconcili.│
│ service  │  │link svc  │  │ service  │  │ service  │
└────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
     │              │              │              │
     └──────────────┼──────────────┼──────────────┘
                    │              │
         ┌──────────▼──────────────▼──────────┐
         │      notification-service           │
         │  (subscribes to all event streams)  │
         └────────────────────────────────────┘

┌──────────┐  ┌──────────┐  ┌──────────┐
│  AI      │  │ document │  │analytics │
│ assistant│  │ service  │  │ service  │
└────┬─────┘  └────┬─────┘  └────┬─────┘
     │              │              │
     └──────────────┼──────────────┘
                    │
         ┌──────────▼──────────┐
         │    ai-gateway       │
         │  (guardrails)       │
         └────────────────────┘
```

### 4.1 Dependency Rules

| Rule | Description |
|---|---|
| **DEP-001** | Every service depends on `platform-proto` (compiled protobuf types) |
| **DEP-002** | Every service depends on `platform-logging` and `platform-metrics` |
| **DEP-003** | Event-sourced services depend on `platform-event-store` and `platform-outbox` |
| **DEP-004** | Services storing credentials depend on `platform-kms` |
| **DEP-005** | No circular dependencies between domain services |
| **DEP-006** | `api-gateway` depends only on `platform-proto` — it routes, never owns domain logic |

---

## 5. Service Classification

### 5.1 By Data Pattern

| Pattern | Services | Description |
|---|---|---|
| **Event-Sourced** | `orchestration`, `reconciliation`, `subscription`, `invoice` | Full event store with projections. Commands append events; projections build read models. |
| **CRUD + Events** | `operator`, `iam`, `compliance`, `merchant-acquirer-link`, `payment-link`, `document` | Standard CRUD with domain events published for downstream consumers. |
| **Cross-Cutting** | `api-gateway`, `ai-gateway`, `saga-coordinator`, `outbox-relay` | No domain data ownership. Provide infrastructure capabilities. |
| **Pure Consumer** | `analytics`, `notification` | Consume events only — never emit commands. |

### 5.2 By Criticality (Checkout Hot Path)

| Tier | Services | Latency Budget | Description |
|---|---|---|---|
| **Tier 0** | `api-gateway`, `orchestration-service`, `connector-gateway`, `risk-service` | < 10s total | On the checkout critical path. Must never degrade. |
| **Tier 1** | `merchant-acquirer-link-service`, `notification-service` | Seconds to minutes | Supporting real-time flows. Can tolerate brief delays. |
| **Tier 2** | All others | Minutes to hours | Background processing. Can tolerate longer delays. |

---

## 6. Communication Patterns

### 6.1 In-Process Calls (Synchronous — gRPC)

```
Caller                                         Callee
──────                                         ──────
API Gateway        ── gRPC (in-process) ──▶    Orchestration Service
Orchestration      ── gRPC (in-process) ──▶    Connector Gateway
Orchestration      ── gRPC (in-process) ──▶    Risk Service
AI Assistant       ── gRPC (in-process) ──▶    Any read model
```

- Uses in-process gRPC (same process, same memory space)
- No serialization overhead (zero-copy shared memory)
- Deadline propagation enforced at every hop (DEADLINE-001–004)

### 6.2 NATS Channels (Asynchronous — Events)

```
Publisher                                     Subscribers
─────────                                     ──────────
Orchestration      ──nats: payment_authorized──▶  Notification Service
                                                Reconciliation Service
                                                Analytics Service
                                                AI Assistant

Reconciliation     ──nats: settlement_matched──▶  Notification Service
                                                Analytics Service

Invoice            ──nats: invoice_paid────────▶  Notification Service
                                                Analytics Service
```

- In-process channels (tokio `broadcast` / `mpsc`) exposing NATS-compatible API
- All payloads protobuf-encoded
- At-least-once delivery, exactly-once effect via idempotent projections
- DLQ with automatic retry + manual replay

### 6.3 Communication Decision Matrix

| Interaction | Style | Protocol | Why Not the Other |
|---|---|---|---|
| Payment authorize/capture/refund | **Synchronous gRPC** | In-process direct call | Must return result within checkout latency budget |
| Risk score before auth | **Synchronous gRPC** | In-process direct call | Risk score needed before routing decision |
| Domain events (post-auth) | **Async NATS** | In-process channel | Multiple consumers; publisher must not block |
| Settlement file ingestion | **Async NATS** | In-process channel | Decouples ingestion cadence from processing |
| AI Assistant reading data | **Synchronous gRPC** | In-process direct call | Read-only queries need response |
| Notification dispatch | **Async NATS** | In-process channel | Naturally asynchronous |
| Document OCR | **Synchronous gRPC** | In-process direct call | Caller needs OCR result to proceed |

---

## 7. Data Flow: Complete Payment Lifecycle

```
1. Merchant API Request
   ─────────────────────
   POST /v1/payment-intents
   Content-Type: application/protobuf
   <CreatePaymentIntentRequest>
       │
       ▼
2. API Gateway (17-api-gateway)
   ─────────────────────────────
   • TLS termination
   • JWT/API key validation (→ IAM service gRPC)
   • Rate limit check (Redis)
   • Protobuf decode + field validation
   • Route to orchestration-service (in-process gRPC)
       │
       ▼
3. Orchestration Service (05-orchestration-service)
   ─────────────────────────────────────────────────
   • Idempotency check (Redis fast path → event store)
   • Create PaymentIntent aggregate (event store append)
   • Publish PaymentCreated event (NATS outbox)
   • Return PaymentIntent to gateway
       │
       ▼
4. Merchant Authorize Request
   ───────────────────────────
   POST /v1/payment-intents/:id/authorize
       │
       ▼
5. Orchestration Service
   ─────────────────────
   • Validate state transition (Created → Authorizing)
   • Get active routing policy
   • Evaluate conditions → select acquirer
   • Check circuit breaker (merchant-acquirer-link)
       │
       ▼
6. Connector Gateway (04-connector-gateway)
   ──────────────────────────────────────────
   • Translate to acquirer-specific format
   • Check 3DS enrollment
   • Send authorization request
   • Receive response
       │
       ▼
7. Orchestration Service
   ─────────────────────
   • If 3DS required → redirect cardholder to ACS
   • If approved → append PaymentAuthorized event
   • If declined → try next acquirer (failover routing)
   • Publish events (NATS outbox)
       │
       ▼
8. Downstream Consumers (async via NATS)
   ──────────────────────────────────────
   • Notification service → send receipt email
   • Reconciliation service → track settlement expectation
   • Analytics service → record for dashboards
   • AI Assistant → index for search
   • Invoice/Subscription service → update related records
```

---

## 8. Workspace Configuration

### 8.1 Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "proto",
    "migrations",
    "services/operator-service",
    "services/iam-service",
    "services/compliance-service",
    "services/connector-gateway",
    "services/orchestration-service",
    "services/invoice-service",
    "services/payment-link-service",
    "services/subscription-service",
    "services/reconciliation-service",
    "services/dispute-service",
    "services/risk-service",
    "services/ai-assistant-service",
    "services/document-service",
    "services/notification-service",
    "services/analytics-service",
    "services/saga-coordinator",
    "services/scheduler",
    "services/merchant-acquirer-link-service",
    "services/outbox-relay",
    "services/api-gateway",
    "services/ai-gateway",
    "services/shared/logging",
    "services/shared/kms",
    "services/shared/event-store",
    "services/shared/outbox",
    "services/shared/health",
    "services/shared/metrics",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
# gRPC framework
tonic = "0.12"
prost = "0.13"
# ORM
sea-orm = { version = "1", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros"] }
# Redis
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
# Message bus (in-process NATS-compatible API)
tokio-stream = "0.1"
# JWT
jsonwebtoken = "9"
# Argon2 for password hashing
argon2 = "0.5"
# Encryption
aes-gcm = "0.10"
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# Error handling
thiserror = "2"
# UUIDs
uuid = { version = "1", features = ["v7", "serde"] }
# Chrono
chrono = { version = "0.4", features = ["serde"] }
# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
metrics = "0.24"
# Proptest
proptest = "1"
# Test containers
testcontainers = "0.23"

[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
```

### 8.2 Rust Toolchain (`rust-toolchain.toml`)

```toml
[toolchain]
channel = "1.85.0"
targets = ["x86_64-unknown-linux-gnu"]
profile = "default"
```

---

## 9. Configuration & Environment

### 9.1 Environment Variables

Every service consumes configuration from environment variables with a consistent naming pattern:

```rust
// config.rs — shared configuration struct
pub struct ServiceConfig {
    // Database
    pub database_url: String,                    // DATABASE_URL
    pub database_pool_size: u32,                 // DATABASE_POOL_SIZE (default: 20)
    pub database_max_lifetime_secs: u64,         // DATABASE_MAX_LIFETIME (default: 1800)

    // Redis
    pub redis_url: String,                       // REDIS_URL
    pub redis_pool_size: u32,                    // REDIS_POOL_SIZE (default: 10)

    // NATS (in-process — config is for outbox relay health)
    pub nats_subject_prefix: String,             // NATS_SUBJECT_PREFIX (default: "events")

    // Service
    pub service_name: String,                    // SERVICE_NAME
    pub listen_addr: String,                     // LISTEN_ADDR (default: "0.0.0.0:9000")
    pub health_listen_addr: String,              // HEALTH_LISTEN_ADDR (default: "0.0.0.0:8081")
    pub graceful_shutdown_timeout_secs: u64,     // GRACEFUL_SHUTDOWN_TIMEOUT (default: 30)

    // Auth
    pub jwt_secret: String,                      // JWT_SECRET
    pub kms_kek_id: String,                      // KMS_KEK_ID

    // Observability
    pub log_level: String,                       // LOG_LEVEL (default: "info")
    pub otlp_endpoint: Option<String>,            // OTLP_ENDPOINT
}
```

### 9.2 Docker Compose (Development)

```yaml
# docker-compose.yml — local development environment
version: "3.9"
services:
  postgres:
    image: postgres:17
    environment:
      POSTGRES_DB: payment_orchestra
      POSTGRES_USER: platform
      POSTGRES_PASSWORD: dev_password
    ports:
      - "5432:5432"
    volumes:
      - pg_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"   # API
      - "9001:9001"   # Console
    environment:
      MINIO_ROOT_USER: platform
      MINIO_ROOT_PASSWORD: dev_password

  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama

volumes:
  pg_data:
  ollama_data:
```

---

## 10. Deployment Topology

```
                         ┌──────────────────┐
                         │   Load Balancer   │
                         │   (e.g., ALB)     │
                         └────────┬─────────┘
                                  │
                    ┌─────────────┼─────────────┐
                    │             │             │
              ┌─────▼─────┐ ┌─────▼─────┐ ┌─────▼─────┐
              │  Replica 1 │ │  Replica 2 │ │  Replica 3 │
              │  (monolith)│ │  (monolith)│ │  (monolith)│
              └─────┬─────┘ └─────┬─────┘ └─────┬─────┘
                    │             │             │
                    └─────────────┼─────────────┘
                                  │
                    ┌─────────────▼─────────────┐
                    │      PostgreSQL (RDS)      │
                    │  + event stores + CRUD db  │
                    └───────────────────────────┘

                    ┌─────────────┐  ┌─────────────┐
                    │  Redis      │  │  MinIO      │
                    │  (ElastiCache)│  │  (S3-compat) │
                    └─────────────┘  └─────────────┘

                    ┌─────────────┐  ┌─────────────┐
                    │  GPU Node   │  │  GPU Node   │
                    │  (Ollama)   │  │  (Ollama)   │
                    └─────────────┘  └─────────────┘
```

### 10.1 Key Deployment Decisions

| Decision | Rationale |
|---|---|
| **Single deployable unit** (modular monolith) | All 22 modules compile into one binary. Simplifies deployment, reduces operational overhead, enables horizontal scaling via replicas. |
| **Horizontal scaling via replicas** | More traffic? Add more replicas behind the load balancer. All modules scale together. |
| **GPU nodes for AI** | `ai-assistant-service` requires GPU inference (Ollama). Deployed on GPU-backed instances, but all other modules share CPU resources. |
| **No service mesh** | All inter-module communication is in-process. No need for mTLS between modules. Only external connections use TLS. |
| **Shared database instance** | All modules share one PostgreSQL instance with per-module schema isolation. Separation is a future scaling option. |

---

## 11. Key Architecture Decisions (ADRs)

| ADR | Decision | Rationale |
|---|---|---|
| **ADR-001** | Modular monolith over microservices | Simplifies deployment, reduces latency (in-process calls), avoids distributed system complexity. Module boundaries can be extracted later if needed. |
| **ADR-002** | REST paths + protobuf bodies over REST JSON or pure gRPC | Developer familiarity of RESTful URLs + type safety and performance of protobuf. Single source of truth (`.proto` files) for both external and internal contracts. |
| **ADR-003** | POST/PATCH/DELETE only (no GET) | All operations use protobuf bodies — including searches. Avoids URL length limits, enables complex nested filter criteria, and keeps auth/middleware consistent. |
| **ADR-004** | In-process NATS over external NATS server | Removes a deployment dependency, reduces latency, simplifies TLS. The NATS-compatible API ensures module boundaries are preserved — can extract to separate services later. |
| **ADR-005** | PostgreSQL event store over specialized event store (EventStoreDB, Kafka) | Reduces infrastructure diversity. PostgreSQL with append-only tables and optimistic concurrency provides strong consistency guarantees. |
| **ADR-006** | Event sourcing for core services only | Orchestration, reconciliation, subscriptions, and invoices get full event sourcing for audit trail. Supporting services use CRUD + events for simplicity. |
| **ADR-007** | BYOK over payment facilitation | Platform is a routing layer, not a payment gateway. Merchants bring their own gateway credentials. Reduces compliance burden, enables multi-gateway routing. |
| **ADR-008** | Rust with SeaORM over Go/Java | Rust provides memory safety, zero-cost abstractions, and strong type system. SeaORM provides compile-time query checking similar to Ent (Go) but idiomatic for Rust. |

---

## 12. Port & Address Convention

Every service module exposes two ports:

| Port | Purpose | Endpoints |
|---|---|---|
| **9000–9099** | Main service port (gRPC) | All business logic endpoints |
| **8081** | Health port (HTTP) | `/healthz`, `/readyz`, `/startupz`, `/healthz/deep` |

Service port allocation:

| Module | Port |
|---|---|
| `api-gateway` | 9000 |
| `ai-gateway` | 9001 |
| `operator-service` | 9010 |
| `iam-service` | 9011 |
| `compliance-service` | 9012 |
| `connector-gateway` | 9020 |
| `orchestration-service` | 9021 |
| `invoice-service` | 9030 |
| `payment-link-service` | 9031 |
| `subscription-service` | 9032 |
| `reconciliation-service` | 9040 |
| `dispute-service` | 9041 |
| `risk-service` | 9050 |
| `ai-assistant-service` | 9060 |
| `document-service` | 9061 |
| `notification-service` | 9070 |
| `analytics-service` | 9080 |
| `merchant-acquirer-link-service` | 9090 |
| `saga-coordinator` (library) | — (compiled into orchestration) |
| `scheduler` (library) | — (compiled into each service) |
| `outbox-relay` (background task) | — (runs inside event-sourced services) |

---

## 13. Tracing Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Service A   │     │  Service B   │     │  Service C   │
│  (gRPC call) │────▶│  (process)   │────▶│  (publish)   │
└──────────────┘     └──────┬───────┘     └──────────────┘
                            │
                            ▼
                    ┌──────────────┐
                    │  NATS Event  │
                    │  (async)     │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Consumer D  │ (linked span, not child)
                    └──────────────┘
```

- **Synchronous path (gRPC)**: Child spans (parent-child relationship, W3C Trace Context propagated via gRPC metadata)
- **Async path (NATS)**: Linked spans (EventEnvelope carries trace_context, consumer creates linked span — the temporal gap is unbounded)
- **All spans**: Export via OTLP to OpenTelemetry Collector

---

## 14. Build & CI Pipeline

```
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│  Lint    │──▶│  Build   │──▶│  Test    │──▶│  Package │
│  (clippy)│   │  (cargo) │   │ (unit +  │   │ (Docker) │
│  (fmt)   │   │          │   │  int)    │   │          │
│  (deny)  │   │          │   │          │   │          │
└──────────┘   └──────────┘   └──────────┘   └──────────┘

CI Stages:
1. cargo fmt --check
2. cargo clippy -- -D warnings
3. cargo deny check (supply chain)
4. cargo audit (vulnerability scan)
5. cargo build --release
6. cargo test (unit + integration)
7. Docker build + push
8. Security scan (Trivy)
```

---

## 15. Services Quick Reference

| # | Module | Bounded Context | Data Pattern | Criticality | Event Source? |
|---|---|---|---|---|---|
| 00 | `shared-types` | Cross-cutting | Value objects | — | — |
| 01 | `operator-service` | BC-01 | CRUD + events | Tier 2 | No |
| 02 | `iam-service` | BC-02 | CRUD + events | Tier 0 (auth) | No |
| 03 | `compliance-service` | BC-03 | Workflow + events | Tier 2 | No |
| 04 | `connector-gateway` | BC-04 | ACL + adapter | **Tier 0** | No |
| 05 | `orchestration-service` | BC-05 | **Event-sourced** | **Tier 0** | **Yes** |
| 06 | `invoice-service` | BC-06 | **Event-sourced** | Tier 1 | **Yes** |
| 07 | `payment-link-service` | BC-07 | CRUD + events | Tier 1 | No |
| 08 | `subscription-service` | BC-08 | **Event-sourced** | Tier 1 | **Yes** |
| 09 | `reconciliation-service` | BC-09 | **Event-sourced** | Tier 1 | **Yes** |
| 10 | `dispute-service` | BC-10 | CRUD + events | Tier 2 | No |
| 11 | `risk-service` | BC-11 | Rules + scoring | **Tier 0** | No |
| 12 | `ai-assistant-service` | BC-12 | RAG pipeline | Tier 2 | No |
| 13 | `document-service` | BC-13 | CRUD + events | Tier 2 | No |
| 14 | `notification-service` | BC-14 | Event consumer | Tier 1 | No |
| 15 | `analytics-service` | BC-15 | Event consumer | Tier 2 | No |
| 16 | `saga-coordinator` (library) | BC-17 | Library pattern | Tier 0 | No |
| — | `scheduler` (library) | Cross-cutting | Background jobs | Tier 2 | No |
| — | `outbox-relay` (task) | Cross-cutting | Event publishing | Tier 0 | No |
| 17 | `api-gateway` | Cross-cutting | Ingress | **Tier 0** | No |
| 18 | `ai-gateway` | Cross-cutting | Guardrails (middleware) | Tier 1 | No |
| 19 | `infrastructure` | Cross-cutting | Outbox, health, etc. | **Tier 0** | No |
| 20 | `grpc-proto-definitions` | Cross-cutting | Contracts | — | — |
| 21 | `merchant-acquirer-link` | **BYOK Core** | CRUD + events | Tier 1 | No |
| 22 | `merchant-connector-onboarding` | Cross-service flow | Process orchestration | Tier 1 | No |

---

## 16. References

| Document | Link | Content |
|---|---|---|
| Shared Types | `00-shared-types.md` | Money, IDs, PaymentStatus, EventEnvelope, protobuf types |
| API Gateway | `17-api-gateway.md` | Endpoint design, rate limiting, auth flow, CORS, TDD tests |
| Cross-Cutting Infrastructure | `19-infrastructure-cross-cutting.md` | Outbox pattern, health checks, leader election, logging, encryption |
| SRS Part 04 | `docs/srs/SRS-Part-04-Microservice-Architecture.md` | Full architecture decisions, service catalog, communication patterns |
| SRS Part 09 | `docs/srs/SRS-Part-09-Database-Design.md` | Database schemas, event store design, Redis cache, RLS policies |
| SRS Part 10 | `docs/srs/SRS-Part-10-APIs-gRPC-Contracts.md` | Full gRPC contract definitions, API conventions |
| SRS Part 11 | `docs/srs/SRS-Part-11-Testing-DevOps-Deployment.md` | CI/CD pipeline, load testing, deployment topology |
| Service Docs | `01-operator-service.md` through `22-merchant-connector-onboarding.md` | Per-service DDD + TDD specifications |

---

*End of Backend Architecture Document.*
