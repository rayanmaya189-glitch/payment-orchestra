# Payment Orchestra — Backend

A **production-grade, event-sourced, microservices-based payment orchestration platform** built in Rust. This is a **pure router** — it never holds, touches, or controls funds. It routes payment *instructions* between merchants, their payment gateways, and their customers.

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                    API Gateway                        │
│         REST + Protobuf → gRPC transcoding            │
├──────────────────────────────────────────────────────┤
│                                                       │
│   ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │
│   │   IAM    │  │ Operator │  │   Orchestration   │   │
│   │ Service  │  │  Service │  │     Service ★     │   │
│   ├──────────┤  ├──────────┤  ├──────────────────┤   │
│   │Compliance│  │ Connector│  │   Reconciliation  │   │
│   │ Service  │  │ Gateway  │  │     Service ★     │   │
│   ├──────────┤  ├──────────┼  ├──────────────────┤   │
│   │ Invoice  │  │PaymentLink│  │   Subscription   │   │
│   │ Service ★│  │ Service  │  │     Service ★     │   │
│   ├──────────┤  ├──────────┤  ├──────────────────┤   │
│   │ Dispute  │  │   Risk   │  │    Notification   │   │
│   │ Service  │  │ Service  │  │     Service       │   │
│   ├──────────┤  ├──────────┤  ├──────────────────┤   │
│   │  AI      │  │ Document │  │    Analytics      │   │
│   │Assistant │  │ Service  │  │     Service       │   │
│   ├──────────┤  ├──────────┤  ├──────────────────┤   │
│   │  Saga    │  │Scheduler │  │ Merchant-Acquirer │   │
│   │Coordinator│  │          │  │  Link Service    │   │
│   ├──────────┤  ├──────────┤  ├──────────────────┤   │
│   │ Outbox   │  │ AI-Gate  │  │Merchant Connector │   │
│   │ Relay    │  │  way     │  │  Onboarding       │   │
│   └──────────┘  └──────────┘  └──────────────────┘   │
│                ★ = Event-sourced                      │
├──────────────────────────────────────────────────────┤
│   NATS JetStream  │  PostgreSQL  │  Redis  │  MinIO  │
└──────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- **Rust** 1.86+ (see `rust-toolchain.toml`)
- **Docker** + **Docker Compose** (for PostgreSQL, Redis, NATS, MinIO, Ollama)
- **Protobuf Compiler** (`protoc`) — required for gRPC proto compilation

### Setup

```bash
# 1. Start infrastructure
docker compose up -d

# 2. Copy environment configuration
cp .env.example .env
# Edit .env with your settings (defaults work for local dev)

# 3. Run database migrations
cargo run --package migrations

# 4. Start a service (example: orchestration-service)
cargo run --package orchestration-service

# Or start all services (in separate terminals):
for svc in services/*/; do
    name=$(basename "$svc")
    cargo run --package "${name//-/_}"
done
```

### Run Tests

```bash
# Run all tests across the workspace
cargo test --workspace

# Run tests for a specific service
cargo test -p orchestration-service

# Run tests with output
cargo test -p iam-service -- --nocapture
```

## Services

| # | Service | Port (gRPC) | Port (Health) | Description | Type |
|---|---------|-------------|---------------|-------------|------|
| 1 | `operator-service` | 9001 | 9101 | Merchant registration, verification, lifecycle | CRUD + Events |
| 2 | `iam-service` | 9002 | 9102 | Auth (JWT/API keys), ABAC, MFA, Maker/Checker | CRUD + Events |
| 3 | `compliance-service` | 9003 | 9103 | KYB, AML monitoring, SAR reports | CRUD + Events |
| 4 | `connector-gateway` | 9004 | 9104 | Acquirer adapters, circuit breaker, 3DS | CRUD + Events |
| 5 | `orchestration-service` | 9005 | 9105 | **PaymentIntent lifecycle, routing, failover** | ★ Event-sourced |
| 6 | `invoice-service` | 9006 | 9106 | Invoice create, send, cancel, payment tracking | ★ Event-sourced |
| 7 | `payment-link-service` | 9007 | 9107 | Hosted payment links, QR codes, Apple/Google Pay | CRUD + Events |
| 8 | `subscription-service` | 9008 | 9108 | Subscription billing, dunning, proration | ★ Event-sourced |
| 9 | `reconciliation-service` | 9009 | 9109 | Settlement matching, T+N tracking, fee variance | ★ Event-sourced |
| 10 | `dispute-service` | 9010 | 9110 | Chargeback lifecycle, evidence, representment | CRUD + Events |
| 11 | `risk-service` | 9011 | 9111 | Fraud scoring, rules engine, 3DS exemption | CRUD + Events |
| 12 | `ai-assistant-service` | 9012 | 9112 | RAG pipeline, natural-language Q&A | CRUD + Events |
| 13 | `document-service` | 9013 | 9113 | Document upload, OCR, secure retrieval | CRUD + Events |
| 14 | `notification-service` | 9014 | 9114 | Email/SMS, webhook delivery with HMAC signing | CRUD + Events |
| 15 | `analytics-service` | 9015 | 9115 | Metrics, reporting, analytics queries | CRUD + Events |
| 16 | `saga-coordinator` | 9016 | 9116 | Multi-step cross-aggregate sagas | Library |
| 17 | `scheduler` | 9017 | 9117 | Background jobs, cron, leader election | — |
| 18 | `merchant-acquirer-link-service` | 9018 | 9118 | **BYOK Core** — credential management, health | CRUD + Events |
| 19 | `outbox-relay` | 9019 | 9119 | Transactional outbox → NATS publisher | — |
| 20 | `api-gateway` | 9020 | 9120 | REST + Protobuf ingress, auth, rate limit | — |
| 21 | `ai-gateway` | 9021 | 9121 | AI guardrails, prompt screening, quotas | — |
| 22 | `merchant-connector-onboarding` | 9022 | 9122 | BYOK flow: create link, test, activate | — |

## Shared Crates

| Crate | Description |
|---|---|
| `platform-error` | Error type hierarchy with gRPC status conversion |
| `shared-types` | Money, PaymentStatus, DeclineReason, etc. |
| `platform-middleware` | Auth (JWT+RS256/HS256, API key), ABAC, rate limiting, SSRF protection |
| `platform-logging` | Structured JSON logging + OpenTelemetry OTLP tracing |
| `platform-config` | Environment-based configuration |
| `platform-db` | PostgreSQL connection pooling with per-service databases |
| `platform-messaging` | NATS JetStream event bus with graceful NoopEventBus fallback |
| `platform-metrics` | Prometheus metrics, uptime tracking |
| `platform-health` | Liveness, readiness, deep health checks, gRPC health service |
| `platform-registry` | etcd service discovery |
| `platform-event-store` | Event sourcing with corruption detection |
| `platform-outbox` | Transactional outbox pattern |
| `platform-proto` | Protobuf/gRPC service definitions |
| `platform-vault` | Secrets management |
| `platform-kms` | Key management service |
| `platform-api` | Shared API utilities |
| `platform-clients` | gRPC client connections |

## Key Design Decisions

- **Pure Router, No Custody** — The platform never holds funds. It routes payment *instructions*. Funds flow directly between customer, gateway, and merchant bank.
- **BYOK (Bring Your Own Key)** — Merchants connect their own gateway credentials. The platform is a routing layer, not a payment facilitator.
- **Event Sourcing** — Orchestration, reconciliation, subscriptions, and invoices use event sourcing for full audit trail and temporal queries.
- **REST Paths + Protobuf Bodies** — External APIs use RESTful URL paths with protobuf-encoded request/response bodies. Internal service-to-service uses native gRPC.
- **ABAC, not RBAC** — Attribute-based access control enforced at command handler level.
- **Graceful Degradation** — Every service falls back to in-memory storage if PostgreSQL is unavailable, and NoopEventBus if NATS is unavailable.
- **Transactional Outbox** — Event publishing uses the outbox pattern for reliability (NATS + PostgreSQL).

## Production Configuration

All services are configured via environment variables. See `.env.example` for the full list.

Key variables:
```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/payment_orchestra
REDIS_URL=redis://redis:6379
NATS_URL=nats://nats:4222
JWT_SECRET=change-me-to-a-random-64-char-hex-string
RUST_LOG=info
```

For production:
- Set `JWT_PUBLIC_KEY_PEM` for RS256 JWT validation (AUTH-017)
- Set `SSRF_ALLOW_HTTP=0` (it's 0 by default)
- Set `OTEL_EXPORTER_OTLP_ENDPOINT` for distributed tracing
- Configure per-service database URLs with `{SERVICE}_DATABASE_URL`

## License

MIT
