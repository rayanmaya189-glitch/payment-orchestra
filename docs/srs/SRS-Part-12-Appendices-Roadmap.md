# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 12 of 12:** Appendices, Consolidated Open Questions & Implementation Roadmap
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 12 of 12 — Appendices & Roadmap (FINAL PART OF SERIES) |
| Depends On | All Parts 1–11 |
| Purpose | Close out the series: full traceability matrix, consolidated glossary, consolidated open-questions register (every OQ across Parts 1–11 in one place with owners), and the MVP → Enterprise implementation roadmap. |

---

## 1. Document Series Index

| Part | Title | Core Contribution |
|---|---|---|
| 1 | Vision, Business Requirements, Scope, Stakeholders | Why the product exists; BIZ-xxx register; the no-custody constraint |
| 2 | Business Processes & Use Cases | UC-xxx catalog with full flows, tied to BIZ-xxx |
| 3 | DDD & Bounded Contexts | 16 bounded contexts, 5 core aggregates, 22-event domain catalog |
| 4 | Microservice Architecture | 18-service catalog, API/AI Gateway, gRPC/NATS communication matrix |
| 5 | Payment Orchestration Engine | State machine, routing algorithm, idempotency, marketplace-split addendum |
| 6 | AI Payment Assistant & RAG | Model routing, RAG pipeline, guardrails, evaluation harness |
| 7 | Gateway Connector Framework | ACL trait design, capability flags, decline normalization, settlement formats |
| 8 | Identity, Security & Compliance | RBAC/ABAC, secrets/encryption, audit framework, UAE regulatory mapping |
| 9 | Database Design | Postgres/Redis/ClickHouse/OpenSearch/MinIO schemas |
| 10 | APIs & gRPC Contracts | REST conventions, proto contracts, webhook contract, SDK strategy |
| 11 | Testing, DevOps & Deployment | TDD standards, CI/CD, K8s topology, observability, DR |
| 12 | Appendices & Roadmap | This document |

---

## 2. Full Requirement Traceability Matrix

*(A representative, consolidated cross-section; the authoritative per-domain detail lives in each Part's own §9/§10/§12 traceability table — this matrix exists to show the chain end-to-end for the requirements with the deepest cross-part reach.)*

| Business Requirement (Part 1) | Use Case (Part 2) | Bounded Context / Aggregate (Part 3) | Microservice (Part 4) | Deep-Dive Part | Data Layer (Part 9) | API Surface (Part 10) | Test Gate (Part 11) |
|---|---|---|---|---|---|---|---|
| BIZ-010 (configurable routing) | UC-011 | BC-05 / `RoutingPolicy` (AGG-02) | `orchestration-service` (SVC-05) | Part 5 §3 | `event_store` (orchestration DB) | `/v1/routing-policies` | Unit tests on INV-05, E2E UC-011 |
| BIZ-011 (no custody) | UC-080 (marketplace) | Structural absence of platform-owned-balance aggregate; BC-16 ACL | `marketplace-service` (SVC-16) | Part 5 §6 | `sub_merchant_account` tables | `/v1/sub-merchants` | Architecture review (no numeric test possible for an absence) |
| BIZ-012 / GOAL-002 (failover) | UC-020 AF-020a | BC-05 / `PaymentIntent.RoutingAttempt` | `orchestration-service`, `connector-gateway` | Part 5 §3–5 | `event_store`, `payment_events` (ClickHouse) | `AuthorizePaymentIntent` gRPC/REST | E2E failover test, GOAL-002 metric dashboard |
| BIZ-013 (unified reconciliation) | UC-040/041 | BC-09 / `SettlementBatch` | `reconciliation-service` (SVC-09) | Part 5 (settlement events consumed), Part 9 §1 | `event_store` (reconciliation DB), `reconciliation_exception_projection` | `/v1/reconciliation/exceptions` | Idempotent-ingestion property test (INV-06) |
| BIZ-020/021/023 (AI Assistant) | UC-050 | BC-12 (read-only Conformist) | `ai-assistant-service` (SVC-12), `ai-gateway` (SVC-18) | Part 6 | OpenSearch per-tenant index, Postgres citation log | `/v1/assistant/query` | Top-50 eval suite, EVAL-001 regression gate |
| BIZ-030 (tenant isolation) | (cross-cutting) | Shared kernel `TenantId`, PRIN-03 | All services, enforced at API Gateway | Part 4 §7 | Every table's `tenant_id` leading key | (cross-cutting header/context) | SECTEST-001 cross-tenant suite |
| BIZ-040 (immutable audit) | (cross-cutting) | Event sourcing (PRIN-05) + Tier 2 audit log | All services | Part 8 §5 | `event_store`, `audit_log` (DB-004 privilege-enforced) | (audit export endpoints, per-service) | COV-001 on event-sourced invariants |
| BIZ-043 (KYB evidence, not decisioning) | UC-002 | BC-03 / `KybCase` | `compliance-service` (SVC-03) | Part 8 §6 | `kyb_case` tables, `kyb-evidence-{tenant_id}` MinIO bucket | `/v1/kyb-cases` | Integration test against partner ACL mock |

---

## 3. Consolidated Glossary (Extends Part 3 §2)

| Term | Definition |
|---|---|
| ACL (Anti-Corruption Layer) | A translation boundary preventing an external system's model/vocabulary from leaking into the core domain model. |
| Aggregate | A DDD consistency boundary; the unit of transactional state change. |
| Bounded Context | A DDD strategic-design boundary within which a specific model and ubiquitous language apply consistently. |
| CQRS | Command Query Responsibility Segregation — separating the write model (commands/aggregates) from the read model (queries/projections). |
| Custody | Legal/economic control over funds; this platform is architected to never hold it (Part 1 §6). |
| Event Sourcing | Persisting state as an ordered sequence of immutable domain events, with current state derived by replay/fold. |
| Idempotency Key | A caller-supplied token ensuring a repeated request produces the original result rather than a duplicate effect. |
| RAG (Retrieval-Augmented Generation) | Grounding an LLM's answer in retrieved, tenant-specific data rather than relying solely on parametric model knowledge. |
| Routing Policy | The tenant-configured, versioned rule set determining acquirer selection and failover order. |
| Settlement Batch | An ingested set of settlement records from an acquirer/bank, matched against internal payment records. |
| Tenant | An isolated organizational unit (merchant or platform operator) using the system under its own data/config boundary. |

---

## 4. Consolidated Open Questions Register

*(Every OQ/ASSUMP flagged across Parts 1–11, in one place with owner and blocking relationship, so nothing gets lost between parts.)*

| ID | Description | Owner | Blocks |
|---|---|---|---|
| ASSUMP-001 / OQ-001 (Part 1 §6.4) | Confirm no-custody/licensing posture with UAE legal counsel per business model | Legal (STK-014) | Finalizing Part 5 marketplace-split behavior, GA launch |
| OQ-002 (Part 1) | Confirm licensed split-disbursement partner for marketplace mode | Product/Legal | Part 5 §6, Part 16 (marketplace-service) implementation |
| OQ-003 (Part 1) / OQ-016 (Part 7) | Confirm final MVP acquirer/PSP shortlist | Product (STK-007) | Part 7 connector implementation start |
| OQ-004 (Part 1) | Confirm GPU/inference infrastructure budget | Product/Eng leadership | Part 6 model variant selection, Part 11 §6 benchmarking |
| OQ-005 (Part 2) | Finalize dunning retry schedule defaults | Product | UC-031 final configuration defaults |
| OQ-006 (Part 2) | Secondary-approver threshold for AI-suggested reconciliation matches | Compliance (STK-010) | **Resolved in Part 8 §2.2 ABAC-001** — yes, threshold-based dual control |
| OQ-007 (Part 3) | Whether `RiskAssessment` (BC-11) stays a separate context once ML scoring (H3) is designed | Architecture | H3 fraud/risk design spike |
| OQ-008 (Part 3) / OQ-021 (Part 9) | Event retention/archival policy in NATS JetStream and Postgres | Engineering | Part 11 §7 DR-002 DR runbook finalization |
| OQ-009 (Part 4) | Whether risk-service synchronous scoring adds unacceptable checkout latency | Engineering | Part 11 §6 benchmarking spike (OQ-026) |
| OQ-010 (Part 4) / OQ-022 (Part 9) | Shared vs. separate Postgres instances for invoice/payment-link services | Engineering/Infra | Part 11 infrastructure-as-code templates |
| OQ-011 (Part 5) | Default/configurable range for hard failover hop ceiling | Engineering | Part 11 §6.2 PERF-002 |
| OQ-012 (Part 5) | Which MVP connectors support native idempotency vs. status-check fallback | Engineering | Depends on OQ-003/OQ-016 resolution |
| OQ-013 (Part 6) | Finalize top-50 AI Assistant question list | Product + Finance-Ops persona input | Part 6 §6 evaluation harness content |
| OQ-014 (Part 6) | Confirm GPU hardware spec/quantity | Engineering/Infra | Part 6 NFR-AI-001 numeric targets, Part 11 §6 |
| OQ-015 (Part 6) | Bounded conversation-history window size | Product/UX | Part 6 SESS-001 finalization |
| OQ-017 (Part 7) | Per-connector-per-tenant unique webhook URLs vs. shared disambiguated URL | Engineering | Part 9/10 webhook contract finalization |
| OQ-018 (Part 8) | Exact financial-record retention period | Legal | Part 8 §5.2 AUD-001, Part 11 §7 DR retention alignment |
| OQ-019 (Part 8) | Applicability/mechanics of PDPL erasure requests vs. financial retention | Legal | Any future "data deletion" API design |
| OQ-020 (Part 9) | Confirm BGE-M3 embedding dimension for deployed variant | Engineering/AI | Part 9 §4.2 OpenSearch index template finalization |
| OQ-023 (Part 10) | Final API deprecation-window duration | Product/Legal | Enterprise merchant contract terms |
| OQ-024 (Part 10) | Final SDK language priority order | Product | Part 10 §4 SDK roadmap sequencing |
| OQ-025 (Part 10) | Machine-client auth header scheme | Engineering | Part 10 §1.3 finalization against chosen gateway tech |
| OQ-026 (Part 11) | Run PERF-001 benchmarking spike | Engineering | Converts every latency/capacity placeholder across Parts 5/6/11 into committed numbers |
| OQ-027 (Part 11) | Set RPO/RTO numeric targets | Product/Compliance/Engineering | Part 11 §7 DR runbook finalization |
| OQ-028 (Part 11) | Duration of manual-approval-gated production releases | Engineering leadership | Part 11 §3.1 CI/CD stage 10 maturity |

**Program management note**: Items with a Legal owner (ASSUMP-001/OQ-001, OQ-002, OQ-018, OQ-019) are the highest-priority blockers for GA in any jurisdiction-sensitive configuration, since engineering work can proceed in parallel (feature-flagged, Part 11 REL-002) but must not go live without them resolved.

---

## 5. Implementation Roadmap — MVP → Enterprise

### 5.1 Horizon 1 — MVP / UAE Market Entry

**Goal**: GOAL-001 through GOAL-005 (Part 1 §3.1) achieved; SUCC-001 through SUCC-005 (Part 1 §11) validated.

| Milestone | Key Deliverables | Primary Parts |
|---|---|---|
| M1 — Foundation | `tenant-service`, `iam-service`, `compliance-service` live; UC-001/002 functional | Parts 3, 4, 8, 9 |
| M2 — First Connector | One acquirer connector conformant (Part 7 §5); `connector-gateway` + `orchestration-service` authorize/capture/void/refund functional in sandbox | Parts 5, 7, 9, 10 |
| M3 — Multi-Connector Routing | Two additional connectors; `RoutingPolicy` + failover (UC-011, UC-020 AF-020a) live | Part 5 |
| M4 — Reconciliation | `reconciliation-service` ingesting at least one connector's settlement format; UC-040/041 functional | Parts 7, 9 |
| M5 — Products Layer | Invoice, payment link, subscription billing (PROC-04) live | Part 3 §5.5–5.6 |
| M6 — AI Assistant Baseline | RAG pipeline live against real reconciliation/transaction data; top-50 question set (OQ-013) evaluated and passing EVAL-001 gate | Part 6 |
| M7 — Compliance Hardening | Full audit framework (Part 8 §5), SECTEST-001 cross-tenant suite passing, legal sign-off on custody posture (OQ-001) obtained | Part 8, Part 11 §7 |
| M8 — Pilot GA | First pilot merchant live on production with real acquirer connections | All |

### 5.2 Horizon 2 — GCC Expansion

- Saudi Arabia adapter work (mada scheme, SAMA-relevant reporting) layered onto BC-04's connector framework (Part 7) and BC-09's reconciliation format handling (Part 9) — validates the "adapters, not redesign" claim from Part 1 §2.3 pillar 5.
- Multi-currency reconciliation (BIZ-016) — extends `Money`/FX-provenance value objects (Part 3 PRIN-04).
- Marketplace/sub-merchant orchestration (BIZ-017, Part 3 BC-16, Part 5 §6) — contingent on OQ-002 legal/partner resolution.

### 5.3 Horizon 3 — Platform Maturity

- Success-rate-weighted dynamic routing (GOAL-009, Part 5 §3.3).
- Proactive AI anomaly detection (GOAL-010, Part 6 §7).
- Full SDK ecosystem (GOAL-011, Part 10 §4) across 3+ languages with public developer documentation.

### 5.4 Roadmap Sequencing Principle

- **ROAD-001**: No Horizon 2/3 capability is scheduled ahead of its Horizon 1 prerequisite's production validation (e.g., dynamic routing, H3, is not attempted before static routing/failover, H1, has real production authorization-rate data to weight against) — this is a deliberate "walk before run" discipline consistent with the incremental, testable, TDD-driven engineering culture established in Part 11.

---

## 6. Final Notes on Using This SRS

- This series is a **living specification**. Every Part's Open Items section (consolidated in §4 above) represents known unknowns, not gaps in rigor — they are flagged precisely so they are resolved deliberately (with the right owner) rather than silently assumed away during implementation.
- Where this SRS gives a placeholder (a latency number, a retention period, a hop-count default), it is explicitly marked as such and paired with the mechanism that will produce the real number (a benchmarking spike, a legal opinion, a Product decision) — the intent throughout has been to never present an invented figure with false confidence.
- Recommended next step: convene STK-007 (Product), STK-008 (Engineering), STK-010 (Compliance), and STK-014 (Legal) to walk the §4 consolidated open-questions register and assign near-term resolution deadlines before M1 (§5.1) engineering work begins in earnest.

---

*End of Part 12. End of the 12-Part SRS series for the Multi-Tenant AI-Native Payment Orchestration Platform.*
