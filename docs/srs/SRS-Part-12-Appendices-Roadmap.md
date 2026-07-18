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
| 1 | Vision, Business Requirements, Scope, Stakeholders | Why the product exists; BIZ-xxx register; the no-custody constraint; single-tenant deployment model; Go+Rust language split |
| 2 | Business Processes & Use Cases | UC-xxx catalog with full flows, tied to BIZ-xxx; operator onboarding lifecycle |
| 3 | DDD & Bounded Contexts | 15 bounded contexts + Saga Coordinator (BC-17), SeaORM entities (Rust) + Ent schemas (Go), domain event catalog + outbox pattern |
| 4 | Microservice Architecture | 18-service catalog with Go/Rust language split, API/AI Gateway, gRPC vs NATS decision rules, versioned NATS subjects, outbox relay, circuit breakers, feature flags, graceful degradation framework |
| 5 | Payment Orchestration Engine | State machine, routing algorithm, idempotency, partial auth handling, currency precision, subscription pause/resume (all SeaORM entities) |
| 6 | AI Payment Assistant & RAG | Model routing, RAG pipeline, guardrails, evaluation harness, production quality monitoring, tool use (H2), multi-step reasoning (H2), enhanced prompt injection defense |
| 7 | Gateway Connector Framework | ACL trait design, capability flags, decline normalization, settlement formats, circuit breakers, bulkhead isolation, per-connector retry config |
| 8 | Identity, Security & Compliance | ABAC, secrets/encryption (HSM-backed, AES-256, TLS 1.3), audit framework, UAE regulatory mapping, threat model (STRIDE + abuse cases), OWASP Top 10 controls, PCI-DSS scope minimization, AML/CFT monitoring, fraud scoring, security headers, container hardening, SSRF prevention, PAM, data classification, supply chain security |
| 9 | Database Design | Postgres (SeaORM entities for Rust, Ent schemas for Go), Redis, ClickHouse (Go driver), OpenSearch, MinIO; outbox table, event store archival, connection pooling, RLS policies |
| 10 | APIs & gRPC Contracts | REST conventions, proto contracts (cross-language gRPC), webhook contract, SDK strategy, gRPC service versioning, webhook replay protection, SDK deprecation/migration, API security, NATS subject versioning |
| 11 | Testing, DevOps & Deployment | TDD standards, CI/CD, K8s topology, observability, DR, load testing, chaos engineering, canary deployment, expand-contract migrations, security testing pipeline, penetration testing, PCI-DSS compliance gates |
| 12 | Appendices & Roadmap | This document |

---

## 2. Full Requirement Traceability Matrix

*(A representative, consolidated cross-section; the authoritative per-domain detail lives in each Part's own §9/§10/§12 traceability table — this matrix exists to show the chain end-to-end for the requirements with the deepest cross-part reach.)*

| Business Requirement (Part 1) | Use Case (Part 2) | Bounded Context / Aggregate (Part 3) | Microservice (Part 4) | Deep-Dive Part | Data Layer (Part 9) | API Surface (Part 10) | Test Gate (Part 11) |
|---|---|---|---|---|---|---|---|
| BIZ-010 (configurable routing) | UC-011 | BC-05 / `RoutingPolicy` (AGG-02) | `orchestration-service` (SVC-05) | Part 5 §3 | `event_store` (orchestration DB) | `/v1/routing-policies` | Unit tests on INV-05, E2E UC-011 |
| BIZ-011 (no custody) | Structural absence of platform-owned-balance aggregate | Architecture review (no numeric test possible for an absence) |
| BIZ-012 / GOAL-002 (failover) | UC-020 AF-020a | BC-05 / `PaymentIntent.RoutingAttempt` | `orchestration-service`, `connector-gateway` | Part 5 §3–5 | `event_store`, `payment_events` (ClickHouse) | `AuthorizePaymentIntent` gRPC/REST | E2E failover test, GOAL-002 metric dashboard |
| BIZ-013 (unified reconciliation) | UC-040/041 | BC-09 / `SettlementBatch` | `reconciliation-service` (SVC-09) | Part 5 (settlement events consumed), Part 9 §1 | `event_store` (reconciliation DB), `reconciliation_exception_projection` | `/v1/reconciliation/exceptions` | Idempotent-ingestion property test (INV-06) |
| BIZ-020/021/023 (AI Assistant) | UC-050 | BC-12 (read-only Conformist) | `ai-assistant-service` (SVC-12), `ai-gateway` (SVC-18) | Part 6 | OpenSearch index, Postgres citation log | `/v1/assistant/query` | Top-50 eval suite, EVAL-001 regression gate |
| BIZ-040 (immutable audit) | (cross-cutting) | Event sourcing (PRIN-05) + Tier 2 audit log | All services | Part 8 §5 | `event_store`, `audit_log` (DB-004 privilege-enforced) | (audit export endpoints, per-service) | COV-001 on event-sourced invariants |
| BIZ-043 (KYB evidence, not decisioning) | UC-002 | BC-03 / `KybCase` | `compliance-service` (SVC-03) | Part 8 §6 | `kyb_case` tables, `kyb-evidence` MinIO bucket | `/v1/kyb-cases` | Integration test against partner ACL mock |
| BIZ-044 (OWASP/PCI-DSS compliance) | Cross-cutting | Security controls across all contexts | All services | Part 8 §7, §10 | RLS policies (DB-011), encrypted fields | Security headers (HDR-001), CORS (APISEC-005) | §7.1 SECPIPE-001/002, §7.2 PENTEST, §7.4 PCI |
| BIZ-045 (AML/CFT monitoring) | Cross-cutting | BC-11 Fraud & Risk, compliance-service | `risk-service`, `compliance-service` | Part 8 §11.1 | AML alert queue, SAR generation | `/v1/aml/alerts` | AML rule validation tests |
| BIZ-046 (Fraud scoring) | Cross-cutting | BC-11 Fraud & Risk Scoring | `risk-service` (SVC-11) | Part 5 §3.1 | Risk score cache (Redis) | `/v1/risk/score` | Fraud scoring accuracy tests |
| BIZ-048 (Defense-in-depth) | Cross-cutting | RLS policies, NetworkPolicies, security headers | All services, K8s | Part 8 §7.3, §7.4 | RLS policies (DB-011) | Security headers (HDR-001) | RLS tests, header tests |
| BIZ-049 (Supply chain security) | Cross-cutting | SBOM, dependency pinning, image signing | CI/CD pipeline | Part 8 §7.6 | SBOM artifacts | N/A | Dependency scanning, image signing verification |

---

## 3. Consolidated Glossary (Extends Part 3 §2)

| Term | Definition |
|---|---|
| ACL (Anti-Corruption Layer) | A translation boundary preventing an external system's model/vocabulary from leaking into the core domain model. |
| Aggregate | A DDD consistency boundary; the unit of transactional state change. |
| Bounded Context | A DDD strategic-design boundary within which a specific model and ubiquitous language apply consistently. |
| Checker | The second authorized principal who reviews and approves a change initiated by the Maker. Must be a different person than the Maker (dual-control). |
| CQRS | Command Query Responsibility Segregation — separating the write model (commands/aggregates) from the read model (queries/projections). |
| Custody | Legal/economic control over funds; this platform is architected to never hold it (Part 1 §6). |
| Ent ORM | Go-native ORM for entity lifecycle management (schema generation, migrations, querying) used by Go services. |
| Event Sourcing | Persisting state as an ordered sequence of immutable domain events, with current state derived by replay/fold. |
| gRPC | Google Remote Procedure Call — cross-language RPC framework using Protobuf serialization, used for synchronous service-to-service communication. |
| Idempotency Key | A caller-supplied token ensuring a repeated request produces the original result rather than a duplicate effect. |
| Maker | The first authorized principal who initiates a change. The Maker cannot also be the Checker for the same change. |
| Maker/Checker | A dual-control approval workflow where the Maker initiates a change and a Checker (a different authorized principal) reviews and approves it before the change takes effect. Used for financial, security, and configuration changes. |
| NATS JetStream | Durable, at-least-once message streaming platform used for asynchronous domain event distribution with versioned subjects. |
| RAG (Retrieval-Augmented Generation) | Grounding an LLM's answer in retrieved, tenant-specific data rather than relying solely on parametric model knowledge. |
| Routing Policy | The operator-configured, versioned rule set determining acquirer selection and failover order. |
| SeaORM | Rust-native ORM for entity lifecycle management (schema generation, migrations, querying) used by Rust services. |
| Settlement Batch | An ingested set of settlement records from an acquirer/bank, matched against internal payment records. |
| Timestamp (3-digit ms) | ISO 8601 format with 3-digit millisecond precision: `YYYY-MM-DDTHH:MM:SS.mmmZ`. Used for all timestamps across the system — domain events, audit logs, API responses, database columns, webhook payloads. Enforced at the type level (Rust `DateTimeWithTimeZone`, Go `time.Time`). |
| UUIDv7 | Time-ordered UUID (RFC 9562) used as the primary identifier for all aggregates, entities, and domain events. Provides sequential insert performance on B-tree indexes while retaining distributed-generation benefits. |

---

## 4. Consolidated Open Questions Register

*(Every OQ/ASSUMP flagged across Parts 1–11, in one place with owner and blocking relationship, so nothing gets lost between parts.)*

| ID | Description | Owner | Blocks |
|---|---|---|---|
| ASSUMP-001 / OQ-001 (Part 1 §6.4) | Confirm no-custody/licensing posture with UAE legal counsel per business model | Legal (STK-014) | Finalizing Part 5 marketplace-split behavior, GA launch |
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
| OQ-029 (Part 3) | Finalize saga persistence strategy — same DB vs. dedicated saga DB | Architecture | Part 3 §9.1 SAGA-001 implementation |
| OQ-030 (Part 3) | Determine outbox relay polling interval trade-offs vs. CDC (Debezium) | Engineering | Part 3 §9.2 OUTBOX-001, Part 9 §6.1 DB-006 |
| OQ-031 (Part 3) | Finalize circuit breaker thresholds (error-rate, open-window) against real acquirer data | Engineering | Part 3 §9.3 CB-001 |
| OQ-032 (Part 3) | Confirm archival retention period (default 90 days) against legal retention floor | Legal/Compliance | Part 3 §9.5 ARCH-001, Part 8 AUD-001 |
| OQ-033 (Part 4) | Evaluate Debezium CDC as outbox relay alternative | Engineering | Part 4 §9.1 MOUT-001 |
| OQ-034 (Part 4) | Finalize circuit breaker library choice for Rust | Engineering | Part 4 §9.2 |
| OQ-035 (Part 4) | Confirm feature flag store — Redis-backed vs. off-the-shelf | Engineering/Infra | Part 4 §9.4 MFF-001 |
| OQ-036 (Part 5) | Finalize default PartialAuthorizationPolicy | Product | Part 5 §9.1 PARTIAL-AUTH-002 |
| OQ-037 (Part 5) | Confirm supported currencies and minor-unit precisions for MVP | Product/Engineering | Part 5 §9.2 CURRENCY-001 |
| OQ-038 (Part 5) | Finalize subscription proration calculation method | Product | Part 5 §9.6 SUB-PAUSE-002 |
| OQ-039 (Part 6) | Finalize feedback-loop UX design (thumbs-up/down vs. structured) | Product/UX | Part 6 §9.1 AIMON-001 |
| OQ-040 (Part 6) | Confirm tool-use API surface for H2 | Product/Engineering | Part 6 §9.4 AITOOL-001 |
| OQ-041 (Part 6) | Finalize multi-step reasoning step limit (default 3) | Engineering/AI | Part 6 §9.5 AICHAIN-002 |
| OQ-042 (Part 6) | Evaluate OpenSearch index strategy for single-tenant deployment | Architecture | Part 6 §9.4, Part 9 OS-001 |
| OQ-043 (Part 7) | Finalize circuit breaker thresholds for acquirer connectors | Engineering | Part 7 §5.1 CB-CONN-001 |
| OQ-044 (Part 8) | Finalize KEK rotation schedule (default 90 days) | Security/Compliance | Part 8 §12 SEC-ROT-001 |
| OQ-045 (Part 8) | Confirm API key acquirer scoping for MVP vs. H2 | Product | Part 8 §11 AUTHZ-002 |
| OQ-046 (Part 9) | Finalize outbox relay polling vs. CDC trade-offs | Engineering | Part 9 §6.1 DB-006 |
| OQ-047 (Part 9) | Confirm PgBouncer vs. built-in connection pooler | Engineering | Part 9 §9 POOL-001 |
| OQ-048 (Part 10) | Finalize webhook replay window (default 5 minutes) | Security/Engineering | Part 10 §7 WEBHOOK-REPLAY-001 |
| OQ-049 (Part 10) | Confirm X-SDK-Version header tracking for MVP vs. H2 | Product | Part 10 §8 SDK-DEP-003 |
| OQ-050 (Part 11) | Finalize canary deployment thresholds against real baseline | Engineering/SRE | Part 11 §7.3 CANARY-001 |
| OQ-051 (Part 11) | Confirm chaos engineering tooling choice | Engineering/Infra | Part 11 §7.2 CHAOS-001 |
| OQ-052 (Part 11) | Finalize database migration tooling | Engineering | Part 11 §8.4 MIG-002 |
| OQ-053 (Part 8) | Confirm HSM vs. cloud-native KMS for KEK management | Security/Infra | Part 8 §3 SEC-001, §13 KMP-001 |
| OQ-054 (Part 8) | Finalize SIEM platform selection | Security/Infra | Part 8 §7.5 LOGSEC-003 |
| OQ-055 (Part 8) | Confirm PCI-DSS SAQ type with QSA | Compliance | Part 8 §7.5 SECTEST-005, §4.3 ENC-008 |
| OQ-056 (Part 8) | Finalize red team exercise scope and cadence | Security | Part 8 §10.4 TM-002 |
| OQ-057 (Part 10) | Confirm API request size limits per endpoint type | Product/Security | Part 10 §6.1 APISEC-001 |
| OQ-058 (Part 11) | Confirm PCI-DSS ASV scan provider and schedule | Compliance | Part 11 §7.4 PCI-002 |
| OQ-059 (Part 1) | Finalize AML transaction monitoring rule set | Compliance/Legal | Part 8 §11.1 AML-001 |
| OQ-060 (Part 4) | Finalize gRPC vs NATS decision for compliance-service → orchestration-service (KYB approval notification) | Architecture | Part 4 §4.1 RULE-001/002 |
| OQ-061 (Part 9) | Confirm ClickHouse driver choice for Go (entgo/clickhouse vs native driver) — Ent ORM doesn't support ClickHouse natively | Engineering | Part 9 §3 ClickHouse schema |
| OQ-062 (Part 4) | Finalize NATS subject version deprecation window (default 30 days, NATS-VER-003) | Architecture | Part 4 §4.3 NATS-VER-003 |
| OQ-063 (Part 9) | Confirm SeaORM migration strategy for event-sourced services — forward-only vs reversible migrations | Engineering | Part 9 §1.1 DB migration approach |

**Program management note**: Items with a Legal owner (ASSUMP-001/OQ-001, OQ-018, OQ-019) are the highest-priority blockers for GA. Items from the gap analysis (OQ-029 through OQ-052) represent new engineering decisions that should be resolved during M1–M2 to avoid blocking later milestones. Priority recommendation: resolve OQ-029 (saga persistence), OQ-030 (outbox relay), and OQ-031 (circuit breaker thresholds) before M2 implementation begins, as they are foundational patterns that affect multiple services.

---

## 5. Implementation Roadmap — MVP → Enterprise

### 5.1 Horizon 1 — MVP / UAE Market Entry

**Goal**: GOAL-001 through GOAL-005 (Part 1 §3.1) achieved; SUCC-001 through SUCC-005 (Part 1 §11) validated.

| Milestone | Key Deliverables | Primary Parts |
|---|---|---|
| M1 — Foundation | `operator-service`, `iam-service`, `compliance-service` live; UC-001/002 functional | Parts 3, 4, 8, 9 |
| M2 — First Connector | One acquirer connector conformant (Part 7 §5); `connector-gateway` + `orchestration-service` authorize/capture/void/refund functional in sandbox | Parts 5, 7, 9, 10 |
| M3 — Multi-Connector Routing | Two additional connectors; `RoutingPolicy` + failover (UC-011, UC-020 AF-020a) live | Part 5 |
| M4 — Reconciliation | `reconciliation-service` ingesting at least one connector's settlement format; UC-040/041 functional | Parts 7, 9 |
| M5 — Products Layer | Invoice, payment link, subscription billing (PROC-04) live | Part 3 §5.5–5.6 |
| M6 — AI Assistant Baseline | RAG pipeline live against real reconciliation/transaction data; top-50 question set (OQ-013) evaluated and passing EVAL-001 gate | Part 6 |
| M7 — Compliance Hardening | Full audit framework (Part 8 §5), SECTEST-001 cross-tenant suite passing, legal sign-off on custody posture (OQ-001) obtained | Part 8, Part 11 §7 |
| M8 — Pilot GA | First pilot merchant live on production with real acquirer connections | All |

### 5.2 Horizon 2 — GCC Expansion

- Saudi Arabia adapter work (mada scheme, SAMA-relevant reporting) layered onto BC-04's connector framework (Part 7) and BC-09's reconciliation format handling (Part 9) — validates the "adapters, not redesign" claim from Part 1 §2.3 pillar 4.
- Multi-currency reconciliation (BIZ-016) — extends `Money`/FX-provenance value objects (Part 3 PRIN-04).

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
- The gap analysis additions (Parts 3–11, new sections on sagas, outbox, circuit breakers, threat modeling, load testing, chaos engineering, etc.) address critical design patterns and cross-cutting concerns that were identified during the initial SRS review. These additions strengthen the specification's readiness for implementation without changing the fundamental architecture.
- Recommended next step: convene STK-007 (Product), STK-008 (Engineering), STK-010 (Compliance), and STK-014 (Legal) to walk the §4 consolidated open-questions register and assign near-term resolution deadlines before M1 (§5.1) engineering work begins in earnest.

---

*End of Part 12. End of the 12-Part SRS series for the Multi-Tenant AI-Native Payment Orchestration Platform.*
