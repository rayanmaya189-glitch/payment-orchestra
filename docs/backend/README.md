# Backend Development Documentation

## Overview

This directory contains implementation-ready DDD + TDD specifications for all microservices plus cross-cutting infrastructure and gRPC contracts of the AI-Native Payment Orchestration Platform. Each file provides:

- **Domain Model**: Aggregates, entities, value objects, invariants
- **Commands**: Every mutating operation with preconditions and postconditions
- **Domain Events**: Published events with schemas
- **TDD Test Cases**: Failing test → implementation → passing test
- **gRPC Contracts**: Service definitions (from Part 10)
- **Repository Interface**: Data access patterns
- **Error Catalog**: All error codes for the service

## Architecture

```
BYOK-First, Protobuf-Native, Dual API (REST JSON + Protobuf)
Rust + SeaORM + NATS JetStream + PostgreSQL + Redis + Ollama
```

## Service Index

| File | Service | Bounded Context | Type |
|------|---------|-----------------|------|
| `00-shared-types.md` | Shared | Cross-cutting | Value objects, events, proto defs |
| `01-operator-service.md` | `operator-service` | BC-01 | CRUD + events |
| `02-iam-service.md` | `iam-service` | BC-02 | Auth + ABAC |
| `03-compliance-service.md` | `compliance-service` | BC-03 | KYB + AML workflow |
| `04-connector-gateway.md` | `connector-gateway` | BC-04 | ACL + adapter + circuit breaker + 3DS |
| `05-orchestration-service.md` | `orchestration-service` | BC-05 | **Core** event-sourced + saga lib |
| `06-invoice-service.md` | `invoice-service` | BC-06 | Event-sourced (revised from CRUD) |
| `07-payment-link-service.md` | `payment-link-service` | BC-07 | Public hosted pages |
| `08-subscription-service.md` | `subscription-service` | BC-08 | Event-sourced billing + proration |
| `09-reconciliation-service.md` | `reconciliation-service` | BC-09 | **Core** event-sourced + T+N tracking |
| `10-dispute-service.md` | `dispute-service` | BC-10 | Dispute management + deadline tracking |
| `11-risk-service.md` | `risk-service` | BC-11 | Rule-based scoring + 3DS exemption |
| `12-ai-assistant-service.md` | `ai-assistant-service` | BC-12 | RAG pipeline (GPU-backed) |
| `13-document-service.md` | `document-service` | BC-13 | OCR + document storage |
| `14-notification-service.md` | `notification-service` | BC-14 | Email/SMS + **webhook delivery** |
| `15-analytics-service.md` | `analytics-service` | BC-15 | PostgreSQL analytics (ClickHouse H2) |
| `16-saga-coordinator.md` | Saga Coordinator | BC-17 | Cross-cutting sagas (library pattern) |
| `17-api-gateway.md` | `api-gateway` | Cross-cutting | REST JSON + Protobuf dual ingress |
| `18-ai-gateway.md` | `ai-gateway` | Cross-cutting | AI guardrails (middleware in api-gateway) |
| `19-infrastructure-cross-cutting.md` | Cross-cutting | Infrastructure | Outbox, health, shutdown, leader election, feature flags, logging, pools, degraded modes, secrets, encryption, audit, SSRF |
| `20-grpc-proto-definitions.md` | Cross-cutting | gRPC Contracts | All service proto definitions, shared types, compilation config |
| `21-merchant-acquirer-link-service.md` | `merchant-acquirer-link-service` | **BYOK Core** | **NEW** — owns MerchantAcquirerLink lifecycle, credential management, health monitoring |
| `22-merchant-connector-onboarding.md` | Cross-service flow | **BYOK** | **NEW** — end-to-end BYOK onboarding flow specification (4 services) |

## What Each File Contains

Every service file follows this structure:
1. **Domain Model** — Aggregates, entities, value objects, SeaORM entities
2. **Commands** — Every mutating operation with preconditions/postconditions
3. **Repository Interface** — Data access traits
4. **Error Catalog** — All error codes (HTTP + gRPC status)
5. **TDD Test Cases** — Failing test → implementation → passing test
6. **gRPC Proto** — Service definition in `20-grpc-proto-definitions.md`

## Implementation Order (Revised — No Milestones, Complete Product)

### Phase 1 — Foundation & Identity
1. `00-shared-types.md` (first — everything depends on this)
2. `01-operator-service.md`
3. `02-iam-service.md` (with full MFA, Maker/Checker)
4. `03-compliance-service.md` (KYB + AML monitoring)

### Phase 2 — BYOK Core & Connector Framework
5. **`21-merchant-acquirer-link-service.md`** (NEW — BYOK core)
6. **`22-merchant-connector-onboarding.md`** (NEW — BYOK flow)
7. `04-connector-gateway.md` (with circuit breaker, 3DS, network token, credential management)
8. `05-orchestration-service.md` (with 3DS states, multi-dimensional routing, A/B testing, saga library)

### Phase 3 — Financial Products
9. `09-reconciliation-service.md` (with T+N tracking, fee variance)
10. `06-invoice-service.md` (event-sourced, with tax calc, templates, PDF generation)
11. `07-payment-link-service.md` (with 3DS support, QR codes, Apple Pay/Google Pay)
12. `08-subscription-service.md` (with proration, usage-based billing, account updater)
13. `10-dispute-service.md` (with deadline tracking, automated evidence)
14. `11-risk-service.md` (with 3DS exemption, risk-based routing, negative database)

### Phase 4 — Intelligence & Integration
15. `12-ai-assistant-service.md` (with AI actions, temporal queries, conversation context)
16. `14-notification-service.md` (with merged webhook delivery, template system, delivery analytics)
17. `13-document-service.md`
18. `15-analytics-service.md` (PostgreSQL initially, ClickHouse roadmap)
19. `16-saga-coordinator.md` (library pattern, not separate service)
20. `17-api-gateway.md` (REST JSON + Protobuf dual API)
21. `18-ai-gateway.md` (middleware within api-gateway)
22. `19-infrastructure-cross-cutting.md` (simplified — no hash-linked audit, no separate MinIO requirement)
23. `20-grpc-proto-definitions.md` (all new protos)

## TDD Discipline

Every service follows:
1. Write failing test for invariant/command
2. Implement minimum code to pass
3. Refactor
4. Property-based tests for state machine invariants (proptest)
5. Contract tests for gRPC interfaces
6. Integration tests against testcontainers

## Key Cross-Cutting Concerns

- **BYOK (Bring Your Own Key)**: Merchants connect their own gateway credentials. Platform is a routing layer, NOT a payment gateway or payment facilitator.
- **Event Sourcing**: Orchestration, reconciliation, subscriptions, and invoices use event sourcing for audit trail.
- **3D Secure**: Mandatory for UAE/Europe card payments. Integrated into authorization flow with frictionless and challenge flows.
- **Dual API**: RESTful JSON for merchant adoption + Protobuf-over-HTTP for performance-sensitive use cases. Internal gRPC for service-to-service.
- **Idempotency**: `IdempotencyKey` on ALL mutating commands (not just CreatePaymentIntent). Redis fast-path + event store correctness.
- **Circuit Breaker**: Per-acquirer connection with automatic failover. Closed → Open → Half-Open states.
- **Optimistic Concurrency**: Event store append with expected sequence check.
- **Maker/Checker**: Dual-control for financial/config changes.
- **ABAC**: Attribute-based access control enforced at command handler level.
- **Audit**: Event-sourced contexts: event stream IS audit. Non-event-sourced: append-only audit_log.
