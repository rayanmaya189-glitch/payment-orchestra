# Payment Orchestra

**AI-Native Payment Orchestration Platform** — UAE-First, Multi-Country Ready, BYOK-First.

> Connect once, route everywhere. Intelligent failover, unified reconciliation, AI-powered operations, and compliance-grade audit — without the platform ever holding funds.

---

## Table of Contents

1. [What — The Product](#1-what--the-product)
2. [Why — Business Rationale](#2-why--business-rationale)
3. [Who — Stakeholders & Personas](#3-who--stakeholders--personas)
4. [Where & When — Market & Phases](#4-where--when--market--phases)
5. [Business Model — No Custody, BYOK](#5-business-model--no-custody-byok)
6. [Key Architecture Decisions](#6-key-architecture-decisions)
7. [Pros & Cons Analysis](#7-pros--cons-analysis)
8. [Tech Stack](#8-tech-stack)
9. [Project Structure](#9-project-structure)
10. [Documentation Map](#10-documentation-map)
11. [Getting Started](#11-getting-started)

---

## 1. What — The Product

**Payment Orchestra** is a **pure routing and orchestration layer** — a single-tenant, API-first platform that connects merchants to their existing payment gateways and acquirers. It sits between the merchant, their payment gateways, and their customers — routing transaction instructions, never touching the money.

### How the Money Flows

```
                    Payment Orchestra
                    (Router / Orchestrator)
                          │
            ┌─────────────┼─────────────┐
            │             │             │
            ▼             ▼             ▼
    ┌────────────┐  ┌────────────┐  ┌────────────┐
    │ Payment    │  │ Payment    │  │ Payment    │
    │ Gateway A  │  │ Gateway B  │  │ Gateway C  │
    │ (Stripe,   │  │ (Checkout, │  │ (Network   │
    │  Adyen...) │  │  Telr...)  │  │  Intl...)  │
    └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
          │               │               │
          └───────┬───────┴───────┬───────┘
                  │               │
          ╔═══════╧═══════════════╧═══════════╗
          ║       💰 FUNDS FLOW DIRECTLY       ║
          ║                                     ║
          ║  Customer ──pay──> Payment Gateway  ║
          ║  Payment Gateway ──settle──> Merchant║
          ║                                     ║
          ║  Platform NEVER holds or touches    ║
          ║  the money at any point.            ║
          ╚═════════════════════════════════════╝
```

The platform enables merchants and platforms in the UAE (and expanding to GCC/MENA) to:

- **Connect multiple acquirers/PSPs** through a single integration — one API, one dashboard, one reconciliation view.
- **Route payments intelligently** across providers based on cost, success rate, card scheme, currency, and failover rules.
- **Recover lost revenue** via automatic failover — when the primary acquirer declines or times out, the platform retries on a secondary acquirer.
- **Reconcile automatically** — settlement files from all acquirers are matched against internal order/invoice records.
- **Operate with AI** — a natural-language AI Payment Assistant (RAG over the merchant's own data) answers operational questions, investigates anomalies, and drafts reports.
- **Stay audit-ready** — every money-movement-relevant event is immutably recorded with actor, timestamp, and reason, from day one.

### What This Product Is NOT

| This Platform | It is NOT |
|---|---|
| Payment orchestration & routing layer | ❌ A licensed payment institution, acquirer, or e-money issuer |
| BYOK (Bring Your Own Key) — merchants connect their own gateway credentials | ❌ A payment facilitator that holds funds |
| AI Assistant grounded in the merchant's own operational data | ❌ A general-purpose LLM chatbot |
| UAE-first with GCC expansion adapters | ❌ A card scheme, domestic switch, or AANI/UAEFTS replacement |

---

## 2. Why — Business Rationale

### The Problem

Merchants in the UAE and GCC face a fragmented payment landscape:

| Problem | Business Impact |
|---|---|
| **BIZ-001**: Merchants integrate with 2–5 separate acquirers (Network International, Telr, PayTabs, Checkout.com, local banks), each with different APIs, settlement cycles, and reporting formats | High integration cost, slow time-to-market |
| **BIZ-002**: Reconciliation across acquirers is manual, spreadsheet-heavy, and error-prone | Significant finance-ops effort every month-end |
| **BIZ-003**: No unified real-time view of transaction health across providers | Siloed dashboards, blind spots |
| **BIZ-004**: Smaller PSPs lack engineering capacity to build robust orchestration/failover | Either accept lower reliability or over-invest |
| **BIZ-005**: Anomaly investigation requires manual cross-referencing across systems | Slow incident response, lost revenue |
| **BIZ-006**: Regulatory/audit requirements demand strong audit trails, but most orchestration tools lack compliance-grade logging from day one | Retrofit costs, compliance risk |

### The Solution

Payment Orchestra solves these by providing a **unified control plane** that:

1. Abstracts away the complexity of multi-acquirer integration
2. Automates failover to recover otherwise-lost transactions (target: 15–25% recovery)
3. Automates settlement matching (target: 70% reduction in manual reconciliation time)
4. Provides an AI Assistant that answers operational questions in natural language
5. Provides an immutable, event-sourced audit trail from day one

### Market Context

```
┌─────────────────────────────────────────────────────────────┐
│                  UAE Payments Landscape                      │
├─────────────────────────────────────────────────────────────┤
│ • Strong Visa/Mastercard penetration                        │
│ • Growing domestic instant-payment rail (AANI)              │
│ • Competitive acquiring landscape (6+ major providers)      │
│ • Variable authorization rates across providers             │
│ • Occasional single-provider outages → revenue impact       │
│ • UAE Central Bank RPSS regulation                          │
│ • PDPL data residency expectations                          │
│ • AI adoption in financial ops is nascent — opportunity      │
└─────────────────────────────────────────────────────────────┘
```

**Why UAE-first?** The UAE has a uniquely fragmented acquiring landscape with strong regulatory guardrails, making it the ideal beachhead for a no-custody orchestration platform. Success here validates the model for GCC expansion (KSA, Bahrain, Oman, Kuwait, Qatar) and broader MENA markets.

---

## 3. Who — Stakeholders & Personas

### Primary Stakeholders

| Stakeholder | Interest | Pain Point |
|---|---|---|
| **Merchant Finance/Ops Team** (STK-001) | Fast, accurate reconciliation; clear settlement visibility | Manual spreadsheets, month-end crunch |
| **Merchant Engineering Team** (STK-002) | Clean API-first integration, reliable webhooks, good SDKs | Multi-acquirer integration complexity |
| **Merchant Business Owner** (STK-003) | Revenue recovery via failover, cost transparency | Single-provider dependency risk |
| **Platform Product Management** (STK-007) | Feature prioritization, roadmap alignment | Balancing breadth vs. depth |
| **Platform Engineering** (STK-008) | Buildable, testable, maintainable DDD-aligned architecture | Distributed system complexity |
| **Platform Security/Compliance** (STK-010) | Audit trail completeness, access control, incident response | Compliance retrofit costs |
| **UAE Regulator** (STK-006) | Compliance with RPSS, AML/CFT, data residency | Non-compliant operators |

### Key Personas

| Persona | Role | Primary Need |
|---|---|---|
| **Fatima, Finance Operations Lead** | Mid-size UAE e-commerce merchant | Daily reconciliation confidence, monthly close automation |
| **Rashid, Head of Engineering** | Merchant integrating the platform | Stable APIs, sandbox, clear webhook semantics, idempotency |
| **Omar, Compliance Officer** | Platform operator (internal) | Audit completeness, defensible "no custody" posture |

---

## 4. Where & When — Market & Phases

### Market

| Market | Phase | Focus |
|---|---|---|
| 🇦🇪 UAE | Phase 1 | Card acquiring (Visa/Mastercard), AANI instant payments, 3+ acquirer integrations |
| 🇸🇦 Saudi Arabia | Phase 2 | mada scheme routing, SAMA reporting adapters |
| 🇧🇭 Bahrain, 🇴🇲 Oman, 🇰🇼 Kuwait, 🇶🇦 Qatar | Phase 2 | GCC expansion adapters |
| 🌍 Broader MENA, selected APAC | Phase 3 | Multi-currency, multi-jurisdiction |

### Product Phases

```
Phase 1 ─── Market Entry (UAE)
  ├── Core Infrastructure & Administration
  ├── BYOK Core & Connector Framework
  ├── Payment Processing & Routing
  ├── Reconciliation & Settlement
  ├── Invoices, Payment Links, Subscriptions
  ├── AI Assistant & Document Management
  └── Compliance, Security & Operations

Phase 2 ─── GCC Expansion
  ├── Saudi Arabia adapter (mada, SAMA)
  ├── Multi-currency reconciliation
  └── AANI/UAEFTS instant payments

Phase 3 ─── Platform Maturity
  ├── Success-rate-weighted dynamic routing
  ├── Proactive AI anomaly detection
  └── Full SDK ecosystem (3+ languages)
```

---

## 5. Business Model — No Custody, BYOK

### The "No Custody" Constraint

The platform **never holds merchant or customer funds**. All settlement occurs directly between acquirers/banks and the merchant. The platform:

- Initiates and orchestrates payment instructions via APIs to licensed acquirers/PSPs
- Records what happened (a ledger of *orchestration and reconciliation*, not *fund ownership*)
- Never nets, pools, or commingles merchant funds in an account it controls

**Business rationale:**
- 🟢 Avoids needing a payment institution/money transmitter license in every jurisdiction
- 🟢 Reduces AML/CFT and safeguarding obligations to those of a technology/data processor
- 🟢 Aligns with UAE Central Bank's RPSS regulatory categories
- ⚠️ Requires legal confirmation (ASSUMP-001) — depends on tenant business model

### BYOK (Bring Your Own Key) Model

Unlike traditional payment facilitators or gateways, Payment Orchestra uses a **BYOK** model:

- Merchants bring their own acquirer/PSP credentials
- The platform is a **routing layer**, not a payment gateway or payment facilitator
- Merchants retain their direct relationships with acquirers
- The platform provides the intelligence, routing, failover, reconciliation, and AI layer on top

**Who this is for:**
- Merchants who already have acquiring relationships and want to optimize across them
- Platforms that want to offer multi-acquirer routing without becoming a payment facilitator
- Enterprises that need compliance-grade audit without moving to a new acquiring provider

**Who this is NOT for:**
- Merchants who want a single payment gateway that handles everything (use Stripe, Adyen, etc.)
- Platforms that need payment facilitation (where the platform becomes the merchant of record)

---

## 6. Key Architecture Decisions

This section summarizes the major architectural decisions documented in full in `docs/adr/README.md` (12 Architecture Decision Records) and `docs/backend/23-backend-architecture.md`.

### Decision Summary Table

| Decision | Chosen Approach | Rejected Alternatives | Rationale |
|---|---|---|---|
| **Deployment topology** | Modular monolith (single deployable unit) | Microservices (separate deployable units per BC) | Team size, latency, simplified deployment. Modules can be extracted later if needed |
| **API protocol** | RESTful URL paths + protobuf-encoded bodies | Pure gRPC, REST JSON, GraphQL | Developer familiarity + type safety. `.proto` files are single source of truth |
| **HTTP methods** | POST/PATCH/DELETE only (no GET) | Full REST (GET/POST/PUT/DELETE) | Consistent middleware, complex filter criteria, no URL length limits |
| **Async messaging** | In-process NATS channels (same process) | External NATS server cluster | Removes deployment dependency, reduces latency. Extraction path preserved via NATS-compatible API |
| **Event store** | PostgreSQL (same as operational DB) | EventStoreDB, DynamoDB, Kafka | Transactional consistency, no new infrastructure, known operational profile |
| **Event sourcing** | Core services only (orchestration, reconciliation, subscription, invoice) | All services, or no services | Pragmatic — core money-movement contexts benefit most. CRUD + events is sufficient for others |
| **BYOK model** | Dedicated service (`merchant-acquirer-link-service`) + onboarding flow | Built into connector-gateway | Compliance boundary, credential isolation, health monitoring per link |
| **Language/framework** | Rust + SeaORM | Go, Java, TypeScript | Performance, memory safety, type safety. SeaORM for compile-time query validation |
| **Auth model** | ABAC (Attribute-Based Access Control) | RBAC (Role-Based) | Finer-grained control, regulatory compliance, multi-dimensional permissions |
| **Credentials** | Envelope encryption (DEK + KEK with HSM) | Plaintext storage, single-layer encryption | PCI-DSS compliance, key rotation without re-encrypting all data |

### Architecture Diagram (Simplified)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        External Clients                              │
│  (React Dashboard, Merchant SDK, Mobile Apps)                        │
└─────────────────────────────┬───────────────────────────────────────┘
                              │ POST/PATCH/DELETE (protobuf bodies)
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                     API Gateway (api-gateway)                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │TLS Term. │  │Auth/Authz│  │Rate Limit/CORS│  │Route + Rewrite │  │
│  └──────────┘  └──────────┘  └──────────────┘  └────────────────┘  │
└─────────────────────────────┬────────────────────────────────────────┘
                              │ In-process gRPC (sync) + NATS channels (async)
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                     MODULAR MONOLITH (Single Process)                 │
│                                                                       │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────────┐ │
│  │  Identity   │  │   BYOK     │  │   Payment   │  │  Financial     │ │
│  │  & Access   │  │   Core     │  │ Processing  │  │  Products      │ │
│  │ (BC-01/02)  │  │(BC-04/MAL) │  │ (BC-05/17)  │  │ (BC-06/07/08)  │ │
│  └────────────┘  └────────────┘  └────────────┘  └────────────────┘ │
│                                                                       │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────────┐ │
│  │Reconcil.   │  │  AI/Intel. │  │ Compliance  │  │ Infrastructure │ │
│  │(BC-09/10)  │  │(BC-12/13)  │  │ (BC-03/11)  │  │ (Cross-cut.)   │ │
│  └────────────┘  └────────────┘  └────────────┘  └────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         ▼                    ▼                    ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────────┐
│  PostgreSQL   │   │    Redis     │   │   ClickHouse /   │
│ (Event Store, │   │  (Cache,     │   │   OpenSearch /   │
│  Operational) │   │   Locks)     │   │   MinIO / Ollama │
└──────────────┘   └──────────────┘   └──────────────────┘
```

---

## 7. Pros & Cons Analysis

### Architecture: Modular Monolith vs. Microservices

| Aspect | Modular Monolith (Chosen) | Microservices (Rejected) |
|---|---|---|
| **🚀 Pros** | • Single binary to deploy and monitor | • Independent scaling per service |
| | • Zero network latency for inter-module calls | • Independent deployment cycles |
| | • Simplified CI/CD (one pipeline) | • Technology diversity per service |
| | • Easier local development and debugging | • Stronger isolation boundaries |
| | • No distributed tracing complexity | • Team autonomy |
| | • Faster development velocity | |
| **⚠️ Cons** | • Single process scales as a unit | • Complex CI/CD (22 pipelines) |
| | • No independent module deployment | • Network latency for every call |
| | • Requires code-level discipline for module boundaries | • Service mesh configuration overhead |
| | • Language lock-in (Rust only) | • Distributed debugging complexity |
| | | • Higher operational cost (K8s, mesh, etc.) |

### API Design: REST Paths + Protobuf Bodies

| Aspect | REST Paths + Protobuf (Chosen) | Pure gRPC | REST JSON |
|---|---|---|---|
| **🚀 Pros** | • Developer-friendly URL structure | • Strong typing everywhere | • Universal tooling (curl, browser) |
| | • Strong typing via `.proto` files | • Native streaming support | • No code generation needed |
| | • Single source of truth (`.proto` file) | • Highest performance | • Lowest friction for new integrations |
| | • No duplication between API spec and implementation | | |
| **⚠️ Cons** | • Requires protobuf code generation | • Less developer-friendly (no curl) | • Runtime-typed, no schema enforcement |
| | • Less familiar to pure REST developers | • Poor browser support | • Verbose payloads |
| | | • Harder debugging | • No compile-time contract checking |

### BYOK Model

| Aspect | BYOK (Chosen) | Payment Facilitation |
|---|---|---|
| **🚀 Pros** | • No payment institution license needed | • Merchant gets one contract, one settlement |
| | • Merchants retain existing acquirer relationships | • Platform controls the full experience |
| | • Lower regulatory burden | • Higher revenue potential (interchange share) |
| | • Faster time-to-market | • Easier merchant onboarding (no existing relationship needed) |
| **⚠️ Cons** | • Merchant must already have (or obtain) acquiring relationships | • Requires payment institution license in each jurisdiction |
| | • Onboarding is more complex (credential setup) | • Higher regulatory capital requirements |
| | • Less control over the end-to-end payment experience | • Full AML/CFT obligations as funds-holder |

### Event Sourcing: Core Services Only

| Aspect | Core-Only Event Sourcing (Chosen) | Full Event Sourcing | No Event Sourcing |
|---|---|---|---|
| **🚀 Pros** | • Immutable audit trail for money movement | • Uniform data pattern everywhere | • Simplest implementation |
| | • Event replay for reconciliation/disputes | • Full temporal query capability | • Lowest storage cost |
| | • Pragmatic — not over-engineered | | • Standard CRUD tooling |
| | • CRUD for non-critical contexts | | |
| **⚠️ Cons** | • Two data patterns (event-sourced + CRUD) | • Higher storage volume | • No built-in audit trail |
| | • Eventual consistency complexity for core contexts | • More complex queries | • Hard to replay/rebuild state |
| | | • Steeper learning curve | • Difficult dispute investigation |

### Language: Rust + SeaORM

| Aspect | Rust + SeaORM (Chosen) | Go | Java/Spring |
|---|---|---|---|
| **🚀 Pros** | • Memory safety without GC | • Fast compile times | • Mature ecosystem |
| | • Zero-cost abstractions | • Excellent concurrency primitives | • Massive talent pool |
| | • Strong type system prevents entire classes of bugs | • Simple, readable syntax | • Rich library ecosystem |
| | • SeaORM gives compile-time query validation | | • Well-known patterns |
| **⚠️ Cons** | • Steeper learning curve | • No compile-time query validation | • Heavy runtime (JVM) |
| | • Slower compile times | • Less memory-safe by default | • GC pauses for latency-sensitive paths |
| | • Fewer Rust-specific payment libraries | • Weaker type system (no Option/Result enforcement) | • Verbose boilerplate |

### Decision: Why These Trade-offs Were Accepted

The overarching philosophy: **build a complete, production-grade product from day one** — not an MVP to be retrofitted later. Every decision prioritizes:

1. **Regulatory compliance** — The no-custody/BYOK model and event-sourced audit trail are non-negotiable for UAE financial services
2. **Latency** — In-process calls keep the checkout hot path under strict 10-second deadlines
3. **Developer experience for merchants** — RESTful URLs with protobuf bodies balance familiarity with type safety
4. **Operational simplicity** — A single binary (modular monolith) is dramatically simpler to operate than 22 microservices
5. **Future extraction** — All decisions preserve the option to extract modules into separate services later without a rewrite

---

## 8. Tech Stack

### Backend

| Component | Technology | Purpose |
|---|---|---|
| **Language** | Rust (edition 2024) | Performance, memory safety, type safety |
| **ORM** | SeaORM | Async, compile-time query validation, migrations |
| **Async Runtime** | Tokio | Industry-standard async runtime |
| **Synchronous IPC** | In-process gRPC (tonic) | Service-to-service calls with protobuf contracts |
| **Async Messaging** | In-process NATS channels | Event distribution with outbox pattern |
| **Primary Database** | PostgreSQL 16 | Event store, operational data, audit log |
| **Cache** | Redis 7 | Session cache, rate limiter, distributed locks |
| **Analytics** | ClickHouse | Time-series analytics, authorization rate queries |
| **Search** | OpenSearch | RAG vector search, full-text search |
| **Object Storage** | MinIO | Document storage, settlement files, evidence |
| **AI Inference** | Ollama (Qwen3 32B, Qwen3-VL 8B, BGE-M3) | Self-hosted, data-resident AI |
| **Migration** | SeaORM Migrations | Schema versioning, forward-only migrations |

### Frontend

| Component | Technology |
|---|---|
| **Framework** | React 19 |
| **Build Tool** | Vite 8 |
| **Type System** | TypeScript 5.x (strict mode) |
| **Styling** | Tailwind CSS 4 |
| **State Management** | Zustand v5 (client state) + TanStack Query v5 (server state) |
| **API Client** | Axios v1.x + protobuf-es / @bufbuild/connect |
| **Validation** | Zod |
| **Charts** | Recharts |
| **Tables** | TanStack Table v9 |
| **Icons** | Lucide React |
| **i18n** | react-i18next |

### Mobile

| Platform | Language | Framework | Min OS |
|---|---|---|---|
| **iOS** | Swift 6.0+ | SwiftUI + MVVM | iOS 17+ |
| **Android** | Kotlin 2.0+ | Jetpack Compose + Material 3 | API 26+ |

### Infrastructure

| Component | Technology |
|---|---|
| **Container Runtime** | Docker |
| **Orchestration** | Kubernetes (via k8s/ manifests) |
| **API Protocol** | RESTful URL paths + protobuf bodies (POST/PATCH/DELETE) |
| **Service Identity** | mTLS (external connections only), in-process identity propagation (internal) |

---

## 9. Project Structure

```
payment-orchestra/
│
├── README.md                         # ← You are here
├── AGENTS.md                         # Agent instructions for AI coding tools
├── feature.md                        # Complete feature catalog
│
├── docs/
│   ├── adr/                          # Architecture Decision Records (12 ADRs)
│   │   └── README.md
│   ├── srs/                          # 12-Part Software Requirements Specification
│   │   ├── SRS-Part-01-Vision-Business-Scope-Stakeholders.md
│   │   ├── SRS-Part-02-Business-Processes-Use-Cases.md
│   │   ├── SRS-Part-03-DDD-Bounded-Contexts.md
│   │   ├── SRS-Part-04-Architecture-Service-Design.md
│   │   ├── SRS-Part-05-Payment-Orchestration-Engine.md
│   │   ├── SRS-Part-06-AI-Assistant-RAG.md
│   │   ├── SRS-Part-07-Gateway-Connector-Framework.md
│   │   ├── SRS-Part-08-Identity-Security-Compliance.md
│   │   ├── SRS-Part-09-Database-Design.md
│   │   ├── SRS-Part-10-APIs-gRPC-Contracts.md
│   │   ├── SRS-Part-11-Testing-DevOps-Deployment.md
│   │   └── SRS-Part-12-Appendices-Roadmap.md
│   ├── backend/                      # Per-service DDD + TDD specifications
│   │   ├── README.md                 # Backend doc index
│   │   ├── 00-shared-types.md        # Shared value objects, event types, proto defs
│   │   ├── 01-operator-service.md    # BC-01: Operator management
│   │   ├── 02-iam-service.md         # BC-02: Auth, ABAC, API keys
│   │   ├── 03-compliance-service.md  # BC-03: KYB, AML workflow
│   │   ├── 04-connector-gateway.md   # BC-04: Acquirer connector framework, circuit breaker, 3DS
│   │   ├── 05-orchestration-service.md # BC-05: Core payment state machine, routing
│   │   ├── 06-invoice-service.md     # BC-06: Invoice lifecycle (event-sourced)
│   │   ├── 07-payment-link-service.md # BC-07: Payment links, hosted checkout
│   │   ├── 08-subscription-service.md # BC-08: Recurring billing, dunning
│   │   ├── 09-reconciliation-service.md # BC-09: Settlement matching, fee tracking
│   │   ├── 10-dispute-service.md     # BC-10: Chargebacks, representment
│   │   ├── 11-risk-service.md        # BC-11: Fraud scoring, 3DS exemption
│   │   ├── 12-ai-assistant-service.md # BC-12: RAG pipeline, AI Q&A
│   │   ├── 13-document-service.md    # BC-13: OCR, document storage
│   │   ├── 14-notification-service.md # BC-14: Email/SMS, webhook delivery
│   │   ├── 15-analytics-service.md   # BC-15: Authorization rates, fee analysis
│   │   ├── 16-saga-coordinator.md    # BC-17: Saga library pattern
│   │   ├── 17-api-gateway.md         # Cross-cutting: External API ingress
│   │   ├── 18-ai-gateway.md          # Cross-cutting: AI guardrails
│   │   ├── 19-infrastructure-cross-cutting.md # Cross-cutting: Outbox, health, secrets, etc.
│   │   ├── 20-grpc-proto-definitions.md # All gRPC proto definitions
│   │   ├── 21-merchant-acquirer-link-service.md # NEW: BYOK Core
│   │   ├── 22-merchant-connector-onboarding.md # NEW: BYOK flow spec
│   │   └── 23-backend-architecture.md # Backend architecture & folder structure
│   ├── frontend/                     # React dashboard specifications
│   ├── ios/                          # iOS client specifications
│   ├── android/                      # Android client specifications
│   ├── landing-page/                 # Marketing landing page specs
│   ├── analysis/                     # Gap analysis (historical)
│   └── srs/                          # (see above)
│
├── platform-backend/                 # Rust backend implementation workspace
│   ├── Cargo.toml                    # Workspace root
│   ├── Cargo.lock
│   ├── rust-toolchain.toml           # Rust toolchain version
│   ├── deny.toml                     # cargo-deny configuration
│   ├── audit.toml                    # cargo-audit configuration
│   ├── docker-compose.yml            # Local dev environment
│   ├── Dockerfile                    # Production container
│   ├── .dockerignore
│   ├── proto/                        # Protobuf definitions (.proto files)
│   │   ├── common.proto
│   │   ├── operator.proto
│   │   ├── iam.proto
│   │   ├── connector.proto
│   │   ├── orchestration.proto
│   │   ├── invoice.proto
│   │   ├── subscription.proto
│   │   ├── reconciliation.proto
│   │   ├── dispute.proto
│   │   ├── risk.proto
│   │   ├── ai_assistant.proto
│   │   ├── document.proto
│   │   ├── notification.proto
│   │   ├── analytics.proto
│   │   ├── saga.proto
│   │   └── compliance.proto
│   ├── migrations/                   # SeaORM migrations
│   │   └── src/
│   │       ├── lib.rs
│   │       └── m20240101_*.rs        # Migration files (001-022)
│   ├── services/                     # Service implementations (22 modules)
│   │   ├── operator-service/
│   │   ├── iam-service/
│   │   ├── compliance-service/
│   │   ├── connector-gateway/
│   │   ├── orchestration-service/
│   │   ├── invoice-service/
│   │   ├── payment-link-service/
│   │   ├── subscription-service/
│   │   ├── reconciliation-service/
│   │   ├── dispute-service/
│   │   ├── risk-service/
│   │   ├── ai-assistant-service/
│   │   ├── document-service/
│   │   ├── notification-service/
│   │   ├── analytics-service/
│   │   ├── saga-coordinator/         # Library crate
│   │   ├── merchant-acquirer-link-service/
│   │   ├── outbox-relay/             # Background task
│   │   ├── api-gateway/
│   │   ├── ai-gateway/
│   │   └── scheduler/                # Library crate
│   ├── tests/                        # Integration & E2E tests
│   │   ├── common/mod.rs
│   │   ├── operator_service_tests.rs
│   │   ├── iam_service_tests.rs
│   │   ├── connector_gateway_tests.rs
│   │   ├── middleware_tests.rs
│   │   ├── security_tests.rs
│   │   └── production_readiness_tests.rs
│   └── load-tests/                   # K6 load test scripts
│       ├── checkout.js
│       ├── failover.js
│       └── README.md
│
├── k8s/                              # Kubernetes manifests
│   └── base/
│       ├── namespace.yml
│       ├── postgresql.yml
│       ├── redis.yml
│       ├── nats.yml
│       └── service-template.yml
│
└── .github/
    └── workflows/
        └── ci.yml                    # CI pipeline
```

---

## 10. Documentation Map

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Where to Start Reading                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. README.md (this file) — Overview, business context, decisions   │
│                                                                      │
│  2. docs/adr/README.md — 12 Architecture Decision Records            │
│     (Why we chose modular monolith, REST+protobuf, BYOK, etc.)      │
│                                                                      │
│  3. docs/backend/23-backend-architecture.md — Architecture & folders │
│     (Module dependency graph, port convention, deployment topology)  │
│                                                                      │
│  4. docs/backend/README.md — Service index with implementation order │
│                                                                      │
│  5. docs/feature.md — Complete feature catalog (136 features)        │
│                                                                      │
│  6. docs/srs/ — 12-Part SRS (deep-dive into every aspect)            │
│     Start with Part 1 (Vision), then Part 4 (Architecture)           │
│                                                                      │
│  7. docs/backend/XX-*.md — Individual service specs                  │
│     Read the service relevant to your implementation focus           │
│                                                                      │
│  8. platform-backend/ — Rust implementation (when ready to code)     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Documents by Role

| Role | Start Here |
|---|---|
| **Product Manager** | `docs/srs/SRS-Part-01-Vision-Business-Scope-Stakeholders.md` → `docs/feature.md` |
| **Architect** | `docs/adr/README.md` → `docs/backend/23-backend-architecture.md` → `docs/backend/README.md` |
| **Backend Engineer** | `docs/backend/README.md` → relevant service spec → `platform-backend/` |
| **Frontend Engineer** | `docs/frontend/00-architecture.md` → `docs/backend/17-api-gateway.md` |
| **Security/Compliance** | `docs/srs/SRS-Part-08-Identity-Security-Compliance.md` |
| **AI/ML Engineer** | `docs/srs/SRS-Part-06-AI-Assistant-RAG.md` → `docs/backend/12-ai-assistant-service.md` |
| **DevOps/SRE** | `docs/srs/SRS-Part-11-Testing-DevOps-Deployment.md` → `docs/backend/19-infrastructure-cross-cutting.md` |

---

## 11. Getting Started

### For Documentation Readers

```bash
# Start with the big picture
cat docs/adr/README.md          # 12 key decisions with rationale
cat docs/backend/README.md      # Service index and implementation order
cat docs/feature.md             # Complete feature catalog

# Then deep-dive into what matters to you
cat docs/backend/23-backend-architecture.md  # Architecture & folder structure
cat docs/backend/17-api-gateway.md           # API design (REST paths + protobuf)
cat docs/backend/XX-your-service.md          # Specific service spec
```

### For Developers (Platform Backend)

```bash
# Prerequisites
rustup show                         # Auto-installs toolchain from rust-toolchain.toml
cargo install sea-orm-cli

# Local environment
cd platform-backend
docker-compose up -d            # PostgreSQL, Redis, etc.
cargo build                     # Build all workspace members
cargo test                      # Run all tests
```

### Key Concepts to Understand First

1. **BYOK-First**: Merchants bring their own gateway credentials. The platform is a routing/intelligence layer, not a payment gateway. See `docs/backend/21-merchant-acquirer-link-service.md`
2. **REST Paths + Protobuf Bodies**: RESTful URL patterns (`POST /v1/payment-intents`) with protobuf-encoded payloads. No JSON, no GET, no form data. See `docs/backend/17-api-gateway.md`
3. **Modular Monolith**: Single deployable binary containing all 22 domain modules. Module boundaries enforced at code level, not process level. See `docs/backend/23-backend-architecture.md`
4. **Event Sourcing for Core**: Orchestration, reconciliation, subscriptions, and invoices use event sourcing. Other services use CRUD + domain events. See `docs/srs/SRS-Part-05-Payment-Orchestration-Engine.md`
5. **No Custody**: The platform never holds funds. All settlement flows directly between acquirers and merchants. See `docs/srs/SRS-Part-01-Vision-Business-Scope-Stakeholders.md#6-the-no-custody-constraint`

---

*For questions, architecture discussions, or to contribute, refer to the `docs/` hierarchy above. The 12-part SRS series provides complete requirements coverage; the backend service docs provide implementation-ready DDD + TDD specifications.*
