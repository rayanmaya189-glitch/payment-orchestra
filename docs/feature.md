# Feature Specification: AI-Native Payment Orchestration Platform

## 1. Product Summary

A single-tenant, API-first payment orchestration platform that enables merchants in the UAE (and expanding to GCC) to:

1. **Connect once, route everywhere**: Connect multiple acquirers/PSPs and define routing rules without code changes
2. **Intelligent failover**: Automatically retry on secondary acquirers when primary declines
3. **Unified reconciliation**: Automated settlement matching across all acquirers
4. **AI-powered operations**: Natural-language assistant grounded in the merchant's own data
5. **Compliance-grade audit**: Immutable event-sourced audit trail from day one

**Pure Router, No Custody**: The platform is a routing and orchestration layer — it never holds, touches, or controls merchant or customer funds. Money flows directly between the merchant's payment gateways and their bank account. The platform routes transaction instructions between the customer, the merchant's chosen payment gateway, and the merchant's systems — but the actual money never passes through the platform.

## 2. Complete Feature Catalog

### 2.1 Core Infrastructure & Administration

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-001 | Operator registration & email verification | Must | BC-01 |
| F-002 | KYB evidence upload & OCR extraction | Must | BC-03 |
| F-003 | Authentication (email/password + MFA) | Must | BC-02 |
| F-004 | API key management (create, rotate, revoke) | Must | BC-02 |
| F-005 | ABAC role-based access control | Must | BC-02 |
| F-006 | Audit logging (Tier 2 for non-event-sourced) | Must | Cross-cutting |
| F-007 | Event sourcing infrastructure (outbox, relay) | Must | Cross-cutting |
| F-008 | NATS JetStream encryption & mTLS | Must | Infrastructure |
| F-009 | PostgreSQL TDE & connection security | Must | Infrastructure |
| F-010 | Redis authentication & encryption | Must | Infrastructure |
| F-011 | Local development environment (docker-compose) | Must | DX |
| F-012 | Protobuf-only API convention (REST paths + protobuf bodies) | Must | Cross-cutting |
| F-130 | Availability SLO (99.9%+) | Must | Operations |
| F-131 | Data durability guarantee (zero data loss) | Must | Operations |
| F-132 | Minimum TPS target | Must | Operations |
| F-133 | Capacity projections (volume, storage, cost) | Must | Operations |
| F-134 | DR quarterly drill execution | Must | Operations |
| F-135 | PCI-DSS QSA scoping assessment | Must | Compliance |
| F-136 | Infrastructure degraded modes (Redis/NATS/ClickHouse/OpenSearch/MinIO) | Must | Infrastructure |

### 2.2 Gateway Profile Configuration

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-015 | Gateway profile creation (limits, fees, card schemes, currencies) | Must | BC-04 |
| F-016 | Gateway profile limits enforcement (min/max amount, daily/monthly volume) | Must | BC-04 |
| F-017 | Gateway profile fee structure (fixed + percentage + cross-border + FX) | Must | BC-04 |
| F-018 | Gateway profile rate limiting (per-second, per-day) | Must | BC-04 |
| F-019 | Gateway profile monitoring thresholds (success rate, latency) | Must | BC-04 |
| F-020 | Gateway rotation strategy (priority, round-robin, weighted, cost-based, success-rate, volume-capped) | Must | BC-04 |
| F-021 | Order-gateway profile linking (each order records which gateway profile was used) | Must | BC-05 |
| F-022 | Gateway profile analytics (per-gateway metrics, fee comparison, volume distribution) | Must | BC-15 |
| F-023 | Gateway profile bulk operations (bulk limit update, bulk enable/disable) | Must | BC-04 |
| F-024 | Gateway health dashboard (success rate, latency, circuit breaker status) | Must | BC-04 |

### 2.3 Payment Processing & Routing

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-020 | Acquirer connector framework (trait + registry) | Must | BC-04 |
| F-021 | First acquirer connector (sandbox + production) | Must | BC-04 |
| F-022 | Connect acquirer (credentials, validation) | Must | BC-04 |
| F-023 | CreatePaymentIntent command | Must | BC-05 |
| F-024 | AuthorizePaymentIntent command | Must | BC-05 |
| F-025 | CapturePaymentIntent command (auto + manual) | Must | BC-05 |
| F-026 | VoidPaymentIntent command | Must | BC-05 |
| F-027 | RefundPaymentIntent command (full + partial) | Must | BC-05 |
| F-028 | Idempotency (caller-facing + acquirer-facing) | Must | BC-05 |
| F-029 | Optimistic concurrency control | Must | BC-05 |
| F-030 | PaymentMethodToken lifecycle management | Must | BC-05 |
| F-031 | Routing policy configuration (static rules) | Must | BC-05 |
| F-032 | Invalid state transition rejection (exhaustive table) | Must | BC-05 |
| F-033 | Deployment quiesce protocol | Must | BC-05 |
| F-034 | Event signature verification (HMAC) | Must | BC-05 |
| F-035 | Saga coordinator (payment lifecycle saga) | Must | BC-17 |
| F-036 | Card testing abuse prevention | Must | BC-05 |

### 2.4 Multi-Connector & Failover Routing

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-040 | Second & third acquirer connectors | Must | BC-04 |
| F-041 | Failover routing (automatic retry on decline) | Must | BC-05 |
| F-042 | Routing policy: card scheme, currency, amount rules | Must | BC-05 |
| F-043 | Partial authorization handling | Must | BC-05 |
| F-044 | Zero-amount authorization (card verification) | Must | BC-05 |
| F-045 | Scheme compliance monitoring | Should | BC-04 |
| F-046 | Gap: Pre-authorization risk check integration | Must | BC-05, BC-11 |
| F-047 | Gap: Risk-based routing rules | Should | BC-05 |
| F-048 | Gap: Source context on PaymentIntent (payment origin tracking) | Must | BC-05 |

### 2.5 Reconciliation & Settlement

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-050 | Settlement file ingestion (webhook, SFTP, API) | Must | BC-09 |
| F-051 | Settlement record normalization | Must | BC-09 |
| F-052 | Double-entry ledger pattern | Must | BC-09 |
| F-053 | Reconciliation matching (exact + fuzzy + AI-assisted) | Must | BC-09 |
| F-054 | Fee breakdown tracking | Must | BC-09 |
| F-055 | Reconciliation exception queue | Must | BC-09 |
| F-056 | Settlement file security (SFTP, checksum) | Must | BC-09 |
| F-057 | Ledger balance verification (daily job) | Must | BC-09 |
| F-058 | Settlement timing (T+N) tracking and overdue alerting | Must | BC-09 |
| F-059 | Fee variance tracking (estimated vs. actual fee) | Must | BC-09 |
| F-060a | Partial settlement handling | Must | BC-09 |
| F-060b | Refund settlement reconciliation | Must | BC-09 |
| F-060c | Settlement adjustment handling (post-settlement corrections) | Must | BC-09 |

### 2.6 Products: Invoices, Payment Links & Subscriptions

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-060 | Invoice creation & lifecycle | Must | BC-06 |
| F-061 | Invoice duplicate prevention (order reference) | Must | BC-06 |
| F-062 | Payment link creation & hosted checkout page | Must | BC-07 |
| F-063 | Payment link expiry & cleanup | Must | BC-07 |
| F-064 | Payment link URL security (128-bit entropy) | Must | BC-07 |
| F-065 | Subscription plan creation | Must | BC-08 |
| F-066 | Subscription renewal (scheduler + idempotency) | Must | BC-08 |
| F-067 | Subscription cancellation race prevention | Must | BC-08 |
| F-068 | Dunning workflow (retry schedule) | Must | BC-08 |
| F-069 | Chargeback recording & tracking | Must | BC-10 |
| F-070 | Webhook delivery (outbound, HMAC-signed) | Must | Cross-cutting |
| F-071 | Webhook payload versioning | Must | Cross-cutting |
| F-072 | Webhook delivery backpressure & dedup | Must | Cross-cutting |
| F-073 | Notification service (email/SMS) | Must | BC-14 |
| F-074 | Outbound webhook subscription management | Must | Cross-cutting |
| F-075 | Outbound webhook delivery retry with exponential backoff | Must | Cross-cutting |
| F-076 | Outbound webhook delivery audit trail | Must | Cross-cutting |
| F-077 | Payment method token lifecycle (store, expire, revoke) | Must | BC-05 |
| F-078 | Chargeback representment evidence management | Must | BC-10 |
| F-079 | Chargeback representment deadline tracking | Must | BC-10 |
| F-080a | FX rate query integration for cross-border transactions | Should | BC-04 |
| F-080b | Cross-border detection for fee calculation | Should | BC-04 |

### 2.7 AI Assistant & Document Management

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-080 | RAG pipeline (ingestion + retrieval + generation) | Must | BC-12 |
| F-081 | AI bias detection & fairness monitoring | Must | BC-12 |
| F-082 | Hallucination detection (citation + numerical) | Must | BC-12 |
| F-083 | Real-time quality monitoring (hourly sampling) | Must | BC-12 |
| F-084 | RAG retrieval drift detection | Must | BC-12 |
| F-085 | AI output rate limiting per query type | Must | BC-12 |
| F-086 | AI data exfiltration prevention | Must | BC-12 |
| F-087 | AI circuit breaker (degrade to raw-data mode) | Must | BC-12 |
| F-088 | AI gateway guardrails (prompt injection, output) | Must | Cross-cutting |
| F-089 | Document upload & OCR pipeline | Must | BC-13 |
| F-090 | OpenSearch security & tenant isolation | Must | BC-12 |

### 2.8 Compliance, Security & Operations

| Feature ID | Feature | Priority | Bounded Context |
|---|---|---|---|
| F-100 | Audit tamper-evidence (hash chaining) | Must | Cross-cutting |
| F-101 | CSRF protection | Must | Cross-cutting |
| F-102 | Session security (SameSite, HttpOnly, Secure) | Must | Cross-cutting |
| F-103 | Account recovery flow (backup codes) | Must | BC-02 |
| F-104 | Credential access monitoring | Must | Cross-cutting |
| F-105 | IP allowlisting for sensitive operations | Must | Cross-cutting |
| F-106 | gRPC security (reflection disabled, actor signing) | Must | Cross-cutting |
| F-107 | PCI-DSS network diagram | Must | Compliance |
| F-108 | Cardholder data flow diagram | Must | Compliance |
| F-109 | PCI token classification (tokens = cardholder data) | Must | Compliance |
| F-110 | Network segmentation zones (CDE, App, Mgmt) | Must | Infrastructure |
| F-111 | DDoS edge protection | Must | Infrastructure |
| F-112 | HSM disaster recovery plan | Must | Infrastructure |
| F-113 | KEK re-encryption job | Must | Infrastructure |
| F-114 | Encryption key inventory | Must | Infrastructure |
| F-115 | Secrets rotation runbook | Must | Compliance |
| F-116 | Data masking for non-production | Must | Infrastructure |
| F-117 | Data portability export API | Must | Compliance |
| F-118 | Mass API key revocation | Must | BC-02 |
| F-119 | Break-glass support access | Must | BC-02 |
| F-120 | AANI/UAEFTS data model extensions | Must | BC-05 |
| F-121 | PCI-DSS 4.0 targeted risk analysis | Must | Compliance |
| F-122 | Card scheme regulations mapping | Must | Compliance |
| F-123 | Runbooks (10 critical scenarios) | Must | Operations |
| F-124 | Property-based tests (state machine) | Must | Testing |
| F-125 | Contract tests (orchestration ↔ connector) | Must | Testing |
| F-126 | Concurrent payment load tests | Must | Testing |
| F-127 | Payment-specific chaos scenarios | Must | Testing |

## 3. Tech Stack

### Backend
```
Rust + SeaORM + NATS JetStream + PostgreSQL
Redis + ClickHouse + OpenSearch + MinIO + Ollama
```

### Frontend
```
React 19 + React Router DOM 7 + Vite 8
TypeScript 5.x (strict) + Tailwind CSS 4
TanStack React Query v5 + Zustand v5
Axios v1.x + Zod + react-i18next
Recharts + TanStack Table v9 + Lucide React
```

### Frontend Pages

All frontend pages communicate with the backend via protobuf-over-HTTP POST. The React dashboard uses a protobuf client library generated from `.proto` files.

| Page | Route | Auth | Description |
|------|-------|------|-------------|
| Dashboard | `/dashboard` | Yes | Stats cards, charts, real-time updates |
| Payments | `/payments` | Yes | Transaction list with filters (protobuf cursor pagination) |
| Payment Detail | `/payments/:id` | Yes | Transaction detail, routing timeline |
| Reconciliation | `/reconciliation` | Yes | Settlement matching dashboard |
| Exceptions | `/reconciliation/exceptions` | Yes | Unmatched settlement queue |
| Invoices | `/invoices` | Yes | Invoice list and management |
| Subscriptions | `/subscriptions` | Yes | Subscription lifecycle |
| Connectors | `/connectors` | Yes | Acquirer connections |
| Settings | `/settings/*` | Yes | API keys, users, routing, compliance |
| AI Assistant | `/assistant` | Yes | Natural-language Q&A |
| Hosted Checkout | `/pay/:token` | No | PCI-DSS isolated payment page |

**PROTO-FE-001**: Frontend uses protobuf-over-HTTP for all API calls. The `protobuf-es` or `@bufbuild/connect` library handles serialization/deserialization.

**PROTO-FE-002**: The hosted checkout page (`/pay/:token`) is the only page that may use simpler request formats for third-party SDK integration (e.g., Stripe.js-style tokenization redirect). This page communicates with the acquirer directly for card tokenization, not through the platform's protobuf API.

## 4. Non-Functional Requirements Summary

| NFR | Target | Measurement |
|---|---|---|
| Checkout p99 latency | [TBD per pilot] | Synthetic monitoring |
| Availability | 99.9%+ monthly | External probes |
| Data durability | 0% loss for committed events | WAL archiving verification |
| RPO (financial data) | 0 | Backup verification |
| RTO (orchestration) | < 5 minutes | DR drill |
| AI response time | [TBD per GPU spec] | p99 latency |
| Audit completeness | 100% of money-movement events | Daily integrity check |
| Frontend bundle size | < 500KB gzipped | Vite build analysis |
| Lighthouse performance | > 90 | Automated CI check |
| WCAG 2.1 AA | Compliant | Accessibility audit |

## 4. Open Questions (OQ-001 through OQ-100)

See Part 12 §4 for the consolidated open questions register. Key infrastructure decisions:

- OQ-066: NATS/Redis encryption configuration
- OQ-073: PostgreSQL TDE mechanism
- OQ-074: Infrastructure security baselines
- OQ-089: PCI token scope with QSA
- OQ-090: Network segmentation zones
- OQ-092: HSM vendor and DR
