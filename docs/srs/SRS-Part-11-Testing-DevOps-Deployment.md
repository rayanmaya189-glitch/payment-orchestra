# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 11 of 12:** Testing Strategy, DevOps, Deployment & Non-Functional Requirements
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 11 of 12 — Testing, DevOps & Deployment |
| Depends On | All prior parts — this Part operationalizes their NFR previews and process constraints (CONS-002 TDD, Part 1) |
| Feeds Into | Part 12 (roadmap sequencing, final traceability matrix, appendices) |
| Scope | TDD standards, acceptance-criteria framework, CI/CD, containerization/Kubernetes topology, observability, performance/scalability targets, disaster recovery, security/AI-evaluation test gates (consolidating previews from Parts 5, 6, 8). |

---

## 1. Test-Driven Development Standards

### 1.1 Why TDD Here (Restated Business Rationale, Part 1 CONS-002)

TDD is a process constraint chosen because the core domain (money movement, Part 5) has an unusually low tolerance for silent regressions — a routing/idempotency bug is a revenue-integrity and compliance incident, not merely a bug ticket. Writing the test first forces the invariant (Part 3 §6 PRIN-01–05) to be made explicit before implementation, rather than discovered after a production incident.

### 1.2 Test Pyramid

| Layer | Scope | Tooling (Rust) | Ownership |
|---|---|---|---|
| Unit tests | Single aggregate/value-object logic (e.g., `PaymentIntent` state transitions, INV-01–04 invariants) | Built-in `#[test]`, `proptest` for property-based testing of invariants (e.g., "no sequence of valid commands ever produces `captured_amount > authorized_amount`") | Feature developer, written before implementation (TDD-001) |
| Integration tests | Single service against a real (test-container) Postgres/Redis instance; command → event store → projection round-trip | `testcontainers-rs` or equivalent, spinning ephemeral Postgres/Redis per test run | Feature developer |
| Contract tests | Cross-service gRPC contracts (Part 10 §2) and connector conformance suites (Part 7 §5) | Generated stub validation against `.proto` definitions; connector conformance harness against sandbox environments | Service owner + connector owner |
| End-to-end tests | Full use-case flows from Part 2 (e.g., UC-020 authorize-with-failover, across `orchestration-service` + a mocked `connector-gateway` sandbox connector) | Docker Compose or ephemeral K8s namespace spinning the relevant service subset | QA Engineering (STK-012) |
| AI evaluation suite | Top-50 question regression (Part 6 §6), citation validity, tenant-isolation-of-retrieval | Custom harness comparing model output against ground-truth answer set | AI/ML Team (STK-009) + QA |

### 1.3 TDD Workflow Rule

- **TDD-001**: For every new domain command/invariant (Part 3, Part 5), a failing unit test expressing the invariant is committed before the implementation that satisfies it — enforced via code review checklist and, where feasible, CI evidence (test file timestamp/commit-order heuristics are advisory only; the primary enforcement is review discipline, since git history can't perfectly prove test-first authorship, and pretending otherwise would be a false rigor).
- **TDD-002**: Every bug fix in a money-movement-relevant context (BC-05, 08, 09, 10, 16) must include a regression test that fails on the pre-fix code and passes on the post-fix code, committed in the same change.

### 1.4 Coverage Expectations

- **COV-001**: Domain/aggregate logic (Part 3 tactical layer) targets very high line/branch coverage (aspirational ~95%+, tracked but not blindly gamed — a coverage percentage is a signal, not the goal; mutation testing, where CI budget allows, is preferred as a stronger correctness signal for the core `orchestration-service` invariants specifically).
- **COV-002**: Infrastructure/glue code (HTTP handler boilerplate, gRPC transport plumbing) has a lower, more pragmatic coverage bar, since its correctness is better validated by integration/contract tests than exhaustive unit tests of trivial pass-through code.

---

## 2. Acceptance Criteria Framework

### 2.1 Given/When/Then, Traced to Part 2 Use Cases

Every use case from Part 2 is decomposed into acceptance criteria in Given/When/Then form before implementation begins, e.g. (for UC-020 AF-020a, failover):

```
Given a tenant with an active RoutingPolicy listing Acquirer A (priority 1) and Acquirer B (priority 2)
  And Acquirer A is configured to return a retryable decline for a specific test card
When a checkout is attempted with that test card
Then the PaymentIntent transitions through Authorizing (Acquirer A) -> Failed(single-hop) -> Authorizing (Acquirer B) -> Authorized
  And exactly two PaymentAuthorizationAttempted events are recorded
  And the total elapsed time is within the configured latency budget (§6.2)
```

- **AC-001**: Acceptance criteria are the shared artifact between Product (STK-007), Engineering (STK-008), and QA (STK-012) — a use case is not "ready for development" until its acceptance criteria are agreed, and not "done" until every criterion has a passing automated test traceable to it (extends the traceability discipline from Part 2 §12/Part 3 §9 into a testable artifact, not just a documentation cross-reference).

---

## 3. CI/CD Pipeline

### 3.1 Pipeline Stages

1. **Lint/format** (`cargo fmt --check`, `cargo clippy -- -D warnings`) — blocking.
2. **Unit + integration tests** (per service, parallelized) — blocking.
3. **Dependency/vulnerability scan** (Part 8 §7 SECTEST-003) and secret-scanning (Part 8 §3 SEC-002) — blocking.
4. **Contract tests** (gRPC schema compatibility check against the previous version — GRPC-003's evolution rules enforced automatically, failing the build on a detected breaking change without a version bump) — blocking.
5. **Connector conformance suite** (Part 7 §5, run against sandbox environments) — blocking for any change touching `connector-gateway`.
6. **AI evaluation regression gate** (Part 6 §6.3 EVAL-001) — blocking for any change touching `ai-assistant-service`, RAG prompts, or model/version configuration.
7. **Build & push container images** — tagged by commit SHA, signed (image provenance).
8. **Deploy to staging** — automatic on merge to main.
9. **End-to-end smoke suite against staging** — blocking for promotion to production.
10. **Production deploy** — progressive rollout (canary/blue-green, §5.3), gated by manual approval for the first period post-GA, with a path to fully automated promotion once release confidence is established.

### 3.2 Environment Strategy

| Environment | Purpose | Data |
|---|---|---|
| Local/dev | Individual developer work | Synthetic/local fixtures |
| CI (ephemeral) | Automated pipeline runs | Ephemeral test containers, destroyed after run |
| Staging | Pre-production validation, connector sandbox integration | Synthetic tenants + real acquirer *sandbox* credentials only |
| Sandbox (tenant-facing) | Merchant developer (ACT-03) integration testing (Part 1 §9 Persona "Rashid") | Tenant's own sandbox-mode data, isolated from their live data within the same tenant record (a `mode` flag, not a separate tenant, to preserve a single onboarding/config experience — Part 9 schema note for Part 12 appendix) |
| Production | Live traffic | Real tenant data, full Part 8 controls active |

---

## 4. Containerization & Kubernetes Topology

### 4.1 Cluster Shape

- **K8S-001**: Each microservice (Part 4 §1.1) is a separate Deployment with its own resource requests/limits, HorizontalPodAutoscaler thresholds tuned per service's load profile — `orchestration-service` and `connector-gateway` scale on a combination of request-rate and p99-latency signals (given their checkout-critical-path role, Part 5 §8 NFR-ORC-001), while `analytics-service` scales primarily on NATS consumer lag (Part 9 §6 XSTORE-002).
- **K8S-002**: GPU-backed node pool, tainted/labeled so only `ai-assistant-service`'s inference pods (Ollama-hosted Qwen3 32B / Qwen3-VL 8B, Part 6 §2, Part 4 §8) schedule there — general CPU-only services are never scheduled onto GPU nodes, avoiding wasted expensive capacity.
- **K8S-003**: Namespace-per-environment (dev/staging/prod), not namespace-per-tenant — tenant isolation is enforced at the application/data layer (Parts 3, 4, 8, 9), not by giving each tenant its own Kubernetes namespace, which would not scale operationally to a large multi-tenant SaaS base.
- **K8S-004**: Service mesh (Part 4 §8) provides mTLS, and additionally provides the observability hooks (§5) for per-service-pair traffic metrics.

### 4.2 Configuration & Secrets

- Managed via the cluster's native secrets mechanism backed by an external secrets manager (Part 8 §3 SEC-002); no environment-specific config baked into container images — the same image is promoted dev → staging → prod, configured externally, so what is deployed to production is byte-identical to what passed staging validation.

---

## 5. Observability

### 5.1 Logging

- **OBS-001**: Structured (JSON) logs from every service, correlation-ID-tagged (Part 3 §4 envelope's `correlation_id` propagated through log context), shipped to a centralized log aggregation platform (specific vendor/tool selection deferred to infrastructure decision outside this SRS's scope, but the requirement for centralization and correlation-ID tagging is firm).
- **OBS-002**: No sensitive data (raw credentials, full card tokens, full KYB document content) is ever logged — log statements are reviewed for this specifically as part of code review checklist (ties to Part 8 §4.3 PCI-scope discipline extended into observability tooling, which is itself a common source of accidental sensitive-data leakage in real systems).

### 5.2 Metrics

- **OBS-003**: Per-service RED metrics (Rate, Errors, Duration) at minimum; `orchestration-service` additionally exposes business-relevant metrics directly (authorization rate per acquirer, failover-recovery rate — the live operational version of GOAL-002's measurement) so dashboards (UC-070) and alerting share the same underlying metric definitions rather than drifting apart.
- **OBS-004**: NATS consumer lag per stream/consumer (Part 9 §6 XSTORE-002) is a first-class monitored metric feeding both operational alerting and the `as_of` freshness values surfaced via API-007 (Part 10).

### 5.3 Tracing

- **OBS-005**: Distributed tracing (OpenTelemetry-compatible) spanning API Gateway → domain service → connector-gateway → external acquirer call, correlated by the same `correlation_id`/`causation_id` used in the domain event envelope (Part 3 §4) — this deliberately reuses one identifier scheme across logs, traces, and domain events rather than maintaining three separate ID systems that would need manual cross-referencing during an incident investigation.

---

## 6. Performance & Scalability Targets

### 6.1 Approach to Setting Numeric Targets

This SRS deliberately avoids inventing precise numeric SLAs without a benchmarking basis (a common SRS anti-pattern — numbers that look authoritative but were never validated against real infrastructure). Instead, it specifies **the methodology and the placeholders to be filled from a dedicated benchmarking spike**, consistent with how Parts 5 (OQ-011), 6 (OQ-014), and elsewhere have flagged latency/capacity numbers as pending.

### 6.2 Latency Budget Methodology (Checkout Path)

```
Total checkout latency budget (target, tenant-perceived)
  = API Gateway overhead
  + orchestration-service processing (routing decision, event append)
  + connector-gateway translation overhead
  + external acquirer network round-trip  (largest, least controllable component)
  + [x number of failover hops, if applicable] × (above three components)
```

- **PERF-001**: The platform's *own* controllable overhead (everything except the external acquirer round-trip) should be benchmarked and budgeted as a small minority of total checkout latency even in the failover case — the specific millisecond target is set once a benchmarking spike runs against real provisioned infrastructure and real (sandbox) acquirer latencies, rather than guessed here.
- **PERF-002**: RTY-002's hard hop ceiling (Part 5 §3.4, placeholder "3") is *derived from* PERF-001's budget once known, not set independently — the ceiling exists specifically to bound worst-case total latency to an acceptable multiple of a single-hop attempt.

### 6.3 Scalability Targets (Structural, Not Numeric Placeholders)

- **SCALE-001**: Every service in Part 4 §1.1 is independently horizontally scalable (stateless application layer; all state in Postgres/Redis/ClickHouse/OpenSearch/MinIO) — this is a structural guarantee verifiable by architecture review (no in-memory-only state that would break under multi-replica scaling), separate from the specific replica-count numbers which are a capacity-planning exercise against real traffic projections once pilot-merchant volume is known.
- **SCALE-002**: The event-store partitioning (Part 9 §1.1, by `tenant_id, aggregate_type, aggregate_id`) means database scaling can proceed via read replicas for query-heavy projections and, if a single Postgres instance's write throughput becomes the bottleneck at large scale, via tenant-range sharding across multiple Postgres instances — flagged here as an architecture escape hatch that exists because of the tenant-scoped key design, not as an MVP requirement.

---

## 7. Disaster Recovery & Backup

- **DR-001**: Postgres: continuous WAL archiving + periodic base backups, point-in-time-recovery capable, cross-availability-zone replication at minimum, with the specific Recovery Point Objective (RPO)/Recovery Time Objective (RTO) targets to be set jointly by Engineering and Product against acceptable business risk (a near-zero RPO is expected for the event-store databases specifically, given BIZ-040's audit-completeness requirement — losing even a small window of committed financial events is a compliance issue, not just a data-loss inconvenience).
- **DR-002**: ClickHouse/OpenSearch: since these are rebuildable projections (Part 3 §7, Part 9 §6) from the Postgres event stores and NATS JetStream retained streams, their DR strategy can tolerate a coarser RTO (rebuild-from-source is an acceptable recovery path) provided NATS retention (Part 3 OQ-008, Part 9 OQ-021) is sufficient to cover the realistic rebuild window — this is exactly why those two open items must be resolved before DR runbooks can be finalized.
- **DR-003**: MinIO: cross-region or cross-AZ replication for KYB evidence and financial-record-linked buckets specifically (§5, Part 9), given their compliance retention obligations (Part 8 §5.2 AUD-001).
- **DR-004**: Regular (at minimum quarterly) DR drills restoring from backups into an isolated environment and validating application-level correctness (not just "the restore command succeeded") — a backup that has never been test-restored is not a verified backup.

---

## 8. Release & Change Management

- **REL-001**: Semantic versioning for external API surfaces (Part 10 §1.1); internal service versions tracked independently since internal services can be deployed more frequently than the public API surface changes.
- **REL-002**: Feature flags for any H2/H3-scoped capability (Part 1 §7.1) being developed incrementally ahead of its full business/legal readiness (e.g., marketplace splits, Part 1 §6.4 OQ-002) — code can exist and be tested in staging well before it is enabled for any real tenant, decoupling "engineering done" from "legally/commercially launched."
- **REL-003**: Every production deployment is correlated to a change record (what changed, which use cases/requirements it affects per the traceability tables in each Part) — supports both incident post-mortems and the compliance expectation of change auditability (Part 8 §5, extended to infrastructure/code changes, not only domain-data changes).

---

## 9. Traceability

| Requirement | Realized By |
|---|---|
| CONS-002 (Part 1, TDD) | §1 entire section |
| SUCC-003 (Part 1, AI top-50 validated pre-GA) | §1.2 AI evaluation suite row, §3.1 stage 6 |
| SUCC-004 (Part 1, zero cross-tenant leakage pre-GA) | §3.1 stage 3/5 (security scanning), Part 8 §7 SECTEST-001 executed here |
| SUCC-005 (Part 1, 100% audit completeness) | §1.4 COV-001 applied to event-sourced invariants, §7 DR-001 RPO discipline |
| BR-020-2 (Part 2, bounded failover latency) | §6.2 methodology |
| Part 5 OQ-011 | §6.2 PERF-002 (resolution mechanism defined, number pending benchmark) |
| Part 6 OQ-014 | §6 methodology applies equally to GPU capacity planning |
| Part 3 OQ-008 / Part 9 OQ-021 | §7 DR-002 (explicitly blocked on their resolution) |

---

## 10. Open Items Carried Forward

- **OQ-026**: Run the PERF-001 benchmarking spike against provisioned staging infrastructure and real sandbox acquirer latencies to convert every "target to be finalized" placeholder in this Part (and in Parts 5, 6) into a committed number.
- **OQ-027**: Set specific RPO/RTO numeric targets (§7 DR-001) jointly with Product/Compliance once business risk tolerance is agreed — this SRS establishes the *mechanism* (continuous WAL archiving, cross-AZ replication, quarterly drills) but intentionally does not invent the specific hour/minute figures without that input.
- **OQ-028**: Finalize CI/CD blocking-gate stringency for stage 10 (production deploy manual approval, §3.1) — how long manual-approval-gated releases continue before moving to fully automated promotion is an organizational-maturity/risk-appetite decision, not a purely technical one.

---

*End of Part 11. Proceed to Part 12: Appendices & Implementation Roadmap.*
