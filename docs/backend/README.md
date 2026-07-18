# Backend Development Documentation

## Overview

This directory contains implementation-ready DDD + TDD specifications for all 18 microservices of the AI-Native Payment Orchestration Platform. Each file provides:

- **Domain Model**: Aggregates, entities, value objects, invariants
- **Commands**: Every mutating operation with preconditions and postconditions
- **Domain Events**: Published events with schemas
- **TDD Test Cases**: Failing test → implementation → passing test
- **gRPC Contracts**: Service definitions (from Part 10)
- **Repository Interface**: Data access patterns
- **Error Catalog**: All error codes for the service

## Architecture

```
Single-Tenant, Event-Sourced, CQRS
Rust + SeaORM + NATS JetStream + PostgreSQL + Redis + ClickHouse + OpenSearch + MinIO + Ollama
```

## Service Index

| File | Service | Bounded Context | Type |
|------|---------|-----------------|------|
| `00-shared-types.md` | Shared | Cross-cutting | Value objects, events, proto defs |
| `01-operator-service.md` | `operator-service` | BC-01 | CRUD + events |
| `02-iam-service.md` | `iam-service` | BC-02 | Auth + ABAC |
| `03-compliance-service.md` | `compliance-service` | BC-03 | KYB workflow |
| `04-connector-gateway.md` | `connector-gateway` | BC-04 | ACL + adapter |
| `05-orchestration-service.md` | `orchestration-service` | BC-05 | **Core** event-sourced |
| `06-invoice-service.md` | `invoice-service` | BC-06 | CRUD + events |
| `07-payment-link-service.md` | `payment-link-service` | BC-07 | Public hosted pages |
| `08-subscription-service.md` | `subscription-service` | BC-08 | Event-sourced billing |
| `09-reconciliation-service.md` | `reconciliation-service` | BC-09 | **Core** event-sourced |
| `10-dispute-service.md` | `dispute-service` | BC-10 | Event-sourced disputes |
| `11-risk-service.md` | `risk-service` | BC-11 | Rule-based scoring |
| `12-ai-assistant-service.md` | `ai-assistant-service` | BC-12 | RAG pipeline |
| `13-document-service.md` | `document-service` | BC-13 | MinIO + OCR |
| `14-notification-service.md` | `notification-service` | BC-14 | Email/SMS dispatch |
| `15-analytics-service.md` | `analytics-service` | BC-15 | ClickHouse projections |
| `16-saga-coordinator.md` | Saga Coordinator | BC-17 | Cross-cutting sagas |
| `17-api-gateway.md` | `api-gateway` | Cross-cutting | REST ingress |
| `18-ai-gateway.md` | `ai-gateway` | Cross-cutting | AI guardrails |

## Implementation Order (Milestone-Aligned)

### M1 — Foundation
1. `00-shared-types.md` (first — everything depends on this)
2. `01-operator-service.md`
3. `02-iam-service.md`
4. `03-compliance-service.md`

### M2 — First Connector
5. `04-connector-gateway.md`
6. `05-orchestration-service.md` (largest file — core domain)
7. `16-saga-coordinator.md`

### M3 — Multi-Connector Routing
8. (extension of `05-orchestration-service.md` routing rules)

### M4 — Reconciliation
9. `09-reconciliation-service.md`

### M5 — Products Layer
10. `06-invoice-service.md`
11. `07-payment-link-service.md`
12. `08-subscription-service.md`
13. `10-dispute-service.md`
14. `14-notification-service.md`

### M6 — AI Assistant
15. `12-ai-assistant-service.md`
16. `18-ai-gateway.md`
17. `13-document-service.md`

### M7 — Compliance Hardening
18. `11-risk-service.md`
19. `15-analytics-service.md`
20. `17-api-gateway.md`

## TDD Discipline

Every service follows:
1. Write failing test for invariant/command
2. Implement minimum code to pass
3. Refactor
4. Property-based tests for state machine invariants (proptest)
5. Contract tests for gRPC interfaces
6. Integration tests against testcontainers

## Key Cross-Cutting Concerns

- **Event Sourcing**: Parts 3, 5, 9, 10 use event sourcing. Others use CRUD + events.
- **Idempotency**: `IdempotencyKey` on all mutating commands. Redis fast-path + event store correctness.
- **Optimistic Concurrency**: Event store append with expected sequence check.
- **Maker/Checker**: Dual-control for financial/config changes (Part 3 §9.2).
- **ABAC**: Attribute-based access control enforced at command handler level.
- **Audit**: Event-sourced contexts: event stream IS audit. Non-event-sourced: append-only audit_log.
