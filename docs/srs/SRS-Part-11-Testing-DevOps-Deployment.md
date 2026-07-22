# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

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
| Staging | Pre-production validation, connector sandbox integration | Synthetic operators + real acquirer *sandbox* credentials only |
| Sandbox (tenant-facing) | Merchant developer (ACT-03) integration testing (Part 1 §9 Persona "Rashid") | Tenant's own sandbox-mode data, isolated from their live data within the same tenant record (a `mode` flag, not a separate tenant, to preserve a single onboarding/config experience — Part 9 schema note for Part 12 appendix) |
| Production | Live traffic | Real tenant data, full Part 8 controls active |

---

## 4. Containerization & Kubernetes Topology

### 4.1 Cluster Shape

- **K8S-001**: Each microservice (Part 4 §1.1) is a separate Deployment with its own resource requests/limits, HorizontalPodAutoscaler thresholds tuned per service's load profile — `orchestration-service` and `connector-gateway` scale on a combination of request-rate and p99-latency signals (given their checkout-critical-path role, Part 5 §8 NFR-ORC-001), while `analytics-service` scales primarily on NATS consumer lag (Part 9 §6 XSTORE-002).
- **K8S-002**: GPU-backed node pool, tainted/labeled so only `ai-assistant-service`'s inference pods (Ollama-hosted Qwen3 32B / Qwen3-VL 8B, Part 6 §2, Part 4 §8) schedule there — general CPU-only services are never scheduled onto GPU nodes, avoiding wasted expensive capacity.
- **K8S-003**: Namespace-per-environment (dev/staging/prod), keeping the deployment topology simple for single-tenant operation.
- **K8S-004**: Service mesh (Part 4 §8) provides mTLS, and additionally provides the observability hooks (§5) for per-service-pair traffic metrics.
- **K8S-005**: Kubernetes NetworkPolicies: default-deny all ingress/egress at the namespace level; explicit allow-lists per service pair (e.g., `api-gateway` → `orchestration-service` on port 50051, `orchestration-service` → `connector-gateway` on port 50052). No service can communicate with another without an explicit NetworkPolicy.
- **K8S-006**: Pod Security Standards: all pods run under the `restricted` profile — non-root user, read-only root filesystem, no privilege escalation, no host network/IPC/PID namespace sharing.
- **K8S-007**: Container images use distroless or scratch base images (no shell, no package manager). Images are signed using cosign/sigstore and verified by a Kubernetes admission controller before deployment.

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
- **SCALE-002**: The event-store partitioning (by `aggregate_type, aggregate_id`) means database scaling can proceed via read replicas for query-heavy projections and, if a single Postgres instance's write throughput becomes the bottleneck at large scale, via sharding across multiple Postgres instances — flagged here as an architecture escape hatch, not an Phase 1 requirement.

---

## 7. Security Testing & Compliance Gates

### 7.1 Security Testing Pipeline

- **SECPIPE-001**: Security testing is integrated at every stage of the CI/CD pipeline:
  1. **Static Analysis (SAST)**: `cargo-audit`, `cargo-deny`, Clippy security lints — blocking on critical/high findings
  2. **Dependency Scanning**: `cargo-audit` against RustSec advisory database — blocking on any known vulnerability
  3. **Container Scanning**: Trivy/Grype scanning of built container images — blocking on critical/high CVEs
  4. **Secret Scanning**: TruffleHog/gitleaks scanning for committed secrets — blocking on any finding
  5. **Software Bill of Materials (SBOM)**: Generated in SPDX/CycloneDX format for every release — non-blocking but required for compliance

- **SECPIPE-002**: Pre-production security gates:
  1. **Row-Level Security (RLS) tests**: Verify that RLS policies prevent cross-service data access (Part 8 SECTEST-004)
  2. **Authorization bypass tests**: Automated tests attempting unauthorized API access (Part 8 SECTEST-001)
  3. **SSRF validation tests**: Verify URL validation blocks private IP ranges (Part 8 SSRF-001/002)
  4. **Security header tests**: Verify all required security headers are present (Part 8 HDR-001)

### 7.2 Penetration Testing

- **PENTEST-001**: External penetration testing conducted quarterly by a qualified third-party firm, covering:
  - API Gateway endpoints (REST and gRPC-Web)
  - AI Gateway and prompt injection vectors
  - Connector webhook endpoints (inbound)
  - Payment link hosted pages
  - Admin dashboard

- **PENTEST-002**: Penetration test findings are tracked in a vulnerability management system with SLA:
  - Critical (CVSS >= 9.0): Remediate within 24 hours
  - High (CVSS >= 7.0): Remediate within 7 days
  - Medium (CVSS >= 4.0): Remediate within 30 days
  - Low (CVSS < 4.0): Remediate within 90 days or accept risk with documented justification

### 7.3 Red Team Exercises

- **REDTEAM-001**: Semi-annual red team exercises conducted by an external security firm, targeting:
  - Authentication and session management bypass
  - Authorization escalation (horizontal and vertical)
  - AI Assistant data exfiltration attempts
  - Supply chain attack vectors
  - Insider threat scenarios

### 7.4 PCI-DSS Compliance

- **PCI-001**: PCI-DSS Self-Assessment Questionnaire (SAQ) completed annually by a Qualified Security Assessor (QSA). The platform targets SAQ-A or SAQ-A-EP scope depending on final architecture review.
- **PCI-002**: Quarterly Approved Scanning Vendor (ASV) scans of all externally-facing endpoints.
- **PCI-003**: Internal vulnerability scans quarterly, with rescans until passing.
- **PCI-004**: Network segmentation documentation maintained and tested quarterly to confirm CDE isolation.

### 7.5 Compliance Documentation

- **COMPLDOC-001**: The following security documentation must be maintained:
  - Information Security Policy (ISP)
  - Acceptable Use Policy
  - Incident Response Plan (Part 8 IR-001)
  - Business Continuity Plan
  - Disaster Recovery Plan (Part 11 §8)
  - Data Classification Policy (Part 8 ENC-009)
  - Key Management Procedures (Part 8 §13)
  - Vendor Risk Assessment Procedures
  - Security Awareness Training Program

---

## 8. Load Testing & Chaos Engineering

### 8.1 Load Testing Strategy

- **LT-001**: A load testing suite is maintained alongside the application code, using a framework such as `k6` or `Locust`, targeting the following scenarios:
  - **Checkout hot path**: `CreatePaymentIntent` + `AuthorizePaymentIntent` at target TPS (transactions per second), measuring p50/p95/p99 latency.
  - **Failover under load**: Inject primary-acquirer failure during load test, validate that failover completes within latency budget (Part 5 RTY-002).
  - **Settlement ingestion burst**: Simulate month-end settlement file ingestion at 10x normal volume.
  - **AI Assistant concurrent queries**: Sustained concurrent Q&A load against the Ollama inference pool.

- **LT-002**: Load tests run against staging environment with realistic data volumes (synthetic tenant data seeded to represent pilot-merchant scale). Results are compared against latency budgets (Part 5 NFR-ORC-001, Part 6 NFR-AI-001) as a CI gate for performance-sensitive services.

- **LT-003**: Performance regression detection: load test results are stored historically; a CI step compares current-run latency metrics against the baseline and fails the build if p99 latency regresses by more than a configurable threshold (default: 15%).

### 8.2 Chaos Engineering

- **CHAOS-001**: A chaos engineering test suite injects controlled failures in staging:
  - **Acquirer timeout/failure**: Simulate one acquirer returning 500s or timing out; validate failover routing (CB-CONN-001, Part 7).
  - **Postgres failover**: Kill the primary Postgres; validate that the service degrades gracefully (write-path fails fast, read-path serves from replica).
  - **Redis eviction**: Simulate Redis memory pressure; validate that cache misses fall through to Postgres (REDIS-001, Part 9).
  - **NATS partition**: Simulate NATS connectivity loss; validate that outbox relay retries and no events are lost.
  - **MinIO unavailability**: Simulate object storage failure; validate that document uploads queue and retry (MDEG-001, Part 4).
  - **GPU pool saturation**: Simulate Ollama inference overload; validate that AI Gateway circuit-breaks (AIGW-005, Part 4).

- **CHAOS-002**: Chaos tests are run quarterly against staging (not production) as a scheduled pipeline job. Results are documented and shared with STK-011 (DevOps/SRE) and STK-008 (Engineering).

- **CHAOS-003**: Chaos engineering findings feed back into circuit breaker thresholds (Part 3 §9.3), retry configurations (Part 3 §9.4), and graceful degradation modes (Part 4 §9.6) — the chaos suite validates the *configured* resilience, not just the *coded* resilience.

### 8.3 Canary Deployment Specification

- **CANARY-001**: Production deployments use canary rollout: a small percentage of traffic (default: 5%) is routed to the new version while the rest remains on the old version. The canary is monitored for:
  - Error rate increase (threshold: >0.5% increase over baseline)
  - p99 latency increase (threshold: >20% increase over baseline)
  - Business metric anomalies (authorization rate drop, reconciliation failure rate increase)

- **CANARY-002**: If any canary health check fails, the deployment is automatically rolled back (traffic shifts 100% to the old version). The rollback is logged as an incident (Part 8 IR-001 pattern).

- **CANARY-003**: For event-sourced services, canary deployment requires special care: the canary and old version must be able to read each other's event format during the transition window (Part 10 GRPC-003 schema evolution rules). Breaking event schema changes use the versioned subject pattern (Part 4 §4.2, `...v2`) and are deployed as a two-phase rollout (deploy consumers first, then publishers).

- **CANARY-004**: Blue-green deployment is available as an alternative for non-event-sourced services (e.g., `notification-service`, `analytics-service`) where the database schema doesn't require gradual migration.

### 8.4 Database Migration Strategy

- **MIG-001**: All Postgres schema changes use expand-contract migration pattern:
  1. **Expand**: Add new columns/tables (backward-compatible with old application code)
  2. **Deploy**: New application code that writes to and reads from the new schema
  3. **Contract**: Remove old columns/tables (only after old code is fully retired)

- **MIG-002**: Migrations are managed via a migration tool (e.g., `refinery`, `sqlx migrate`) and run as part of the CI/CD pipeline, not manually. Migration scripts are version-controlled and tested in CI against a disposable Postgres instance.

- **MIG-003**: Event store schema changes are additive only (new fields in protobuf payloads, GRPC-003). Breaking changes to event schemas use the versioned subject pattern and are never applied retroactively to existing events in the store.

---

## 9. Disaster Recovery & Backup

- **DR-001**: Postgres: continuous WAL archiving + periodic base backups, point-in-time-recovery capable, cross-availability-zone replication at minimum, with the specific Recovery Point Objective (RPO)/Recovery Time Objective (RTO) targets to be set jointly by Engineering and Product against acceptable business risk (a near-zero RPO is expected for the event-store databases specifically, given BIZ-040's audit-completeness requirement — losing even a small window of committed financial events is a compliance issue, not just a data-loss inconvenience).
- **DR-002**: ClickHouse/OpenSearch: since these are rebuildable projections (Part 3 §7, Part 9 §6) from the Postgres event stores and NATS JetStream retained streams, their DR strategy can tolerate a coarser RTO (rebuild-from-source is an acceptable recovery path) provided NATS retention (Part 3 OQ-008, Part 9 OQ-021) is sufficient to cover the realistic rebuild window — this is exactly why those two open items must be resolved before DR runbooks can be finalized.
- **DR-003**: MinIO: cross-region or cross-AZ replication for KYB evidence and financial-record-linked buckets specifically (§5, Part 9), given their compliance retention obligations (Part 8 §5.2 AUD-001).
- **DR-004**: Regular (at minimum quarterly) DR drills restoring from backups into an isolated environment and validating application-level correctness (not just "the restore command succeeded") — a backup that has never been test-restored is not a verified backup.

### 9.1 Business Continuity Plan

- **BCP-001**: Business Impact Analysis (BIA):

| Service | RTO Target | RPO Target | Impact of Downtime |
|---|---|---|---|
| `orchestration-service` | 5 minutes | 0 (event store) | Payment processing halted — revenue loss |
| `connector-gateway` | 5 minutes | 0 | Payment processing halted — revenue loss |
| `iam-service` | 5 minutes | 0 | Authentication failed — all users locked out |
| `reconciliation-service` | 4 hours | 1 hour | Reconciliation delayed — no immediate revenue impact |
| `analytics-service` | 24 hours | 4 hours | Dashboard data stale — no immediate revenue impact |
| `ai-assistant-service` | 1 hour | 0 | AI assistant unavailable — degraded user experience |
| `notification-service` | 2 hours | 1 hour | Notifications delayed — no immediate revenue impact |
| `invoice-service` | 1 hour | 0 | Invoice generation halted |
| `subscription-service` | 4 hours | 1 hour | Subscription renewals delayed |
| `dispute-service` | 4 hours | 1 hour | Chargeback handling delayed |
| `risk-service` | 5 minutes | 0 | Risk scoring unavailable — transactions may be processed without risk assessment |

- **BCP-002**: Manual fallback procedures:
  - If `orchestration-service` is down, operators can process payments directly through acquirer dashboards (manual fallback) while the platform is restored.
  - If `reconciliation-service` is down, settlement files are queued in MinIO and processed when the service is restored.
  - If `notification-service` is down, notifications are queued in Postgres and sent when the service is restored.

- **BCP-003**: Communication during outages:
  - Status page updated within 5 minutes of detection for SEV-1 incidents
  - Automated email/SMS notifications to affected operators (Part 8 §9.2 IR-COMM-002)
  - Internal Slack channel updated with real-time status

### 9.2 Operational Runbooks

- **RUNBOOK-001**: Each service has a dedicated runbook documenting:
  - Service purpose and dependencies
  - Health check endpoints and expected responses (Part 4 §6.3)
  - Common failure modes and symptoms
  - Step-by-step troubleshooting procedures
  - Escalation contacts and procedures
  - Recovery procedures (restart, failover, rollback)

- **RUNBOOK-002**: Specific runbooks for critical scenarios:
  - **Acquirer outage**: How to detect (circuit breaker open, error rate spike), how to verify failover is working, how to notify operators
  - **Database failover**: How to verify automatic failover completed, how to validate data consistency, how to check replication lag
  - **Redis failover**: How to verify cache is warming, how to check for stale data, how to monitor performance degradation
  - **NATS partition**: How to detect consumer lag, how to verify no events are lost, how to replay missed events
  - **AI model degradation**: How to detect quality drop, how to fallback to raw data queries, how to notify operators
  - **Certificate expiration**: How to detect impending expiration (monitoring), how to rotate (automated + manual procedure)
  - **Secret compromise**: How to revoke (immediate), how to rotate (follow KMP-004), how to notify affected parties

### 9.3 Monitoring & Alerting Specification

- **ALERT-001**: Alert severity levels and routing:

| Severity | Criteria | Notification | Response Time |
|---|---|---|---|
| Critical | Payment processing down, data breach, security incident | PagerDuty + SMS + email | 15 minutes |
| High | Service degraded > 5 minutes, error rate > 1%, security finding CVSS >= 7 | PagerDuty + email | 1 hour |
| Medium | Service degraded < 5 minutes, error rate > 0.1%, performance regression | Email | 4 hours |
| Low | Cosmetic issues, minor bugs, information-only | Dashboard | 24 hours |

- **ALERT-002**: Key alert definitions:
  - `checkout_error_rate > 1% for 5 minutes` → Critical
  - `acquirer_circuit_breaker_open > 5 minutes` → High
  - `nats_consumer_lag > 10000 for 10 minutes` → High
  - `database_connection_pool_exhausted > 5 seconds` → High
  - `ai_model_quality_drop > 10% on regression suite` → High
  - `dlq_depth > 100` → High
  - `certificate_expiration < 30 days` → Medium
  - `backup_age > 24 hours` → Medium

- **ALERT-003**: On-call rotation: weekly rotation with primary and secondary on-call engineers. Escalation: primary → secondary → engineering lead → CISO (for security incidents).

---

## 11. Release & Change Management

### 11.1 Maker/Checker for Production Changes

All production-affecting changes follow the Maker/Checker pattern (Part 3 MKCK-001):

| Change Type | Maker | Checker | Timeout | History Tracked |
|---|---|---|---|---|
| Production deployment | CI/CD pipeline (automated) | Senior Engineer (manual approval) | 4 hours | Deployment log with commit SHA, diff, approver |
| Database migration | Developer (via PR) | Tech Lead + DBA review | 24 hours | Migration log with before/after schema |
| Infrastructure change (Terraform/IaC) | Developer (via PR) | Platform Engineer | 24 hours | Change log with resource diff |
| Secret/credential rotation | Security tool (automated) | Security Admin (manual confirmation) | 12 hours | Rotation log with old/new key references |
| Feature flag activation | Developer | Product Owner | 48 hours | Flag change log with before/after state |
| Hotfix deployment | On-call Engineer | Engineering Lead | 2 hours | Hotfix log with incident reference |

### 11.2 Change History

- **CHG-001**: Every production change is recorded in a `change_history` table (Part 3 MKCK-001) with:
  - Change ID (UUIDv7)
  - Change type (deployment, migration, secret rotation, etc.)
  - Maker (who initiated)
  - Checker (who approved)
  - Status (pending → approved → executed → verified)
  - Diff/snapshot of what changed
  - Timestamp of each status transition
  - Incident correlation (if hotfix)

- **CHG-002**: Change history is immutable — no updates or deletes permitted (append-only, Part 8 AUD-003). Retained for 7 years (financial compliance).

- **CHG-003**: Change history is queryable by: change ID, date range, change type, actor (Maker/Checker), and status — supporting both operational debugging and compliance audit.

### 11.3 Approval Workflow

- **APPROVE-001**: Production deployments require explicit approval in the CI/CD pipeline (Part 11 §3.1 stage 10). The approver must be a different person than the committer (Maker/Checker segregation, Part 3 MKCK-002).
- **APPROVE-002**: Database migrations require both Tech Lead approval (schema correctness) and DBA review (performance, locking, rollback safety). Both must approve before migration runs.
- **APPROVE-003**: Emergency hotfixes follow expedited approval: on-call Engineer can deploy with Engineering Lead approval within 2 hours, with mandatory post-incident review within 48 hours.

### 11.4 Rollback Procedures

- **ROLLBACK-001**: Every production deployment has a documented rollback procedure. Rollback is treated as a new deployment (not an undo) — the rollback is a forward-deploy of the previous version.
- **ROLLBACK-002**: Database migrations must be reversible (expand-contract pattern, Part 11 §8.4 MIG-001). If a migration is not reversible, it must be flagged during the approval process and a manual rollback procedure documented.
- **ROLLBACK-003**: Rollback decisions follow the same Maker/Checker pattern as forward deployments — the on-call Engineer proposes, Engineering Lead approves.

---

## 12. Traceability

| Requirement | Realized By |
|---|---|
| CONS-002 (Part 1, TDD) | §1 entire section |
| SUCC-003 (Part 1, AI top-50 validated pre-GA) | §1.2 AI evaluation suite row, §3.1 stage 6 |
| SUCC-004 (Part 1, zero data leakage pre-GA) | §3.1 stage 3/5 (security scanning), §7.1 SECPIPE-001/002 |
| SUCC-005 (Part 1, 100% audit completeness) | §1.4 COV-001 applied to event-sourced invariants, §9 DR-001 RPO discipline |
| BR-020-2 (Part 2, bounded failover latency) | §6.2 methodology |
| Part 5 OQ-011 | §6.2 PERF-002 (resolution mechanism defined, number pending benchmark) |
| Part 6 OQ-014 | §7 methodology applies equally to GPU capacity planning |
| Part 3 OQ-008 / Part 9 OQ-021 | §9 DR-002 (explicitly blocked on their resolution) |
| Load testing strategy | §8.1 LT-001 through LT-003 |
| Chaos engineering | §8.2 CHAOS-001 through CHAOS-003 |
| Canary/blue-green deployment | §8.3 CANARY-001 through CANARY-004 |
| Database migration strategy (expand-contract) | §8.4 MIG-001 through MIG-003 |
| BIZ-044 (OWASP/PCI-DSS compliance) | §7.1 SECPIPE-001/002, §7.2 PENTEST-001/002, §7.4 PCI-001 through PCI-004 |
| BIZ-048 (defense-in-depth security) | §7.1 SECPIPE-002 RLS tests, authorization tests, SSRF tests |
| BIZ-049 (supply chain security) | §7.1 SECPIPE-001 SBOM, dependency scanning, container scanning |
| BIZ-050 (privileged access management) | §7.3 REDTEAM-001, Part 8 §7.7 PAM |
| OWASP A01-A10 compliance | §7.1 SECPIPE-001/002, §7.2 PENTEST-001/002, §7.3 REDTEAM-001 |

---

## 13. Gap Analysis Additions — Operational Hardening

### 13.1 Alert Fatigue Prevention

**ALERT-FAT-001**: Alert correlation and suppression framework:
- **Alert groups**: Related alerts are grouped under a root cause (e.g., "acquirer-X-down" suppresses circuit breaker, error rate, DLQ depth, and consumer lag alerts for 30 minutes)
- **Runbook linkage**: Every alert has a `runbook_url` field linking to the specific playbook (RUNBOOK-001)
- **Alert review cadence**: Monthly review of alert thresholds, false-positive rates, and alert volume per on-call engineer

**ALERT-FAT-002**: Alert severity escalation: if an alert is not acknowledged within its response time (ALERT-001), it auto-escalates to the next severity level and contacts the escalation chain.

**ALERT-FAT-003**: Alert noise budget: maximum 5 actionable alerts per on-call engineer per week. If exceeded, thresholds are reviewed and adjusted.

### 13.2 Runbook Template & Example Runbooks

**RUNBOOK-TPL-001**: All runbooks follow this template:

```markdown
# Runbook: [Incident Type]

## Detection
- **Alert name**: [ALERT-XXX]
- **Symptoms**: [What the operator sees]
- **Metrics**: [Specific metric thresholds]

## Impact
- **Affected services**: [List]
- **User impact**: [Description]
- **Revenue impact**: [If applicable]

## Triage Steps
1. [Step 1]
2. [Step 2]
3. [Step 3]

## Mitigation Steps
1. [Step 1]
2. [Step 2]

## Validation Steps
1. [How to confirm mitigation worked]
2. [Metrics to watch]

## Escalation
- **Primary**: [On-call engineer]
- **Secondary**: [Engineering lead]
- **Security**: [CISO, for security incidents]

## Post-Incident
- [ ] Create incident report within 48 hours
- [ ] Schedule post-mortem
- [ ] Track remediation items
```

**RUNBOOK-EX-001**: Example runbook — Acquirer Outage:

```markdown
# Runbook: Acquirer Outage

## Detection
- **Alert name**: acquirer_circuit_breaker_open
- **Symptoms**: circuit breaker open for >5 minutes, elevated error rate on checkout path
- **Metrics**: acquirer_error_rate > 50%, checkout_error_rate elevated

## Impact
- **Affected services**: connector-gateway, orchestration-service
- **User impact**: Payments routed to failing acquirer will fail; failover should automatically route to next acquirer
- **Revenue impact**: Potential revenue loss if all acquirers are affected

## Triage Steps
1. Check acquirer status page (if available)
2. Verify circuit breaker state in Redis: `redis-cli GET "cb:{connector_id}"`
3. Check connector-gateway logs for error patterns
4. Verify failover is working: check PaymentAuthorizationAttempted events for multi-hop routing

## Mitigation Steps
1. If single acquirer: confirm failover routing is active (automatic)
2. If multiple acquirers: activate emergency maintenance mode (feature flag FF-EMERGENCY-MAINT)
3. Notify affected operators via status page
4. Contact acquirer support if available

## Validation Steps
1. Monitor checkout_error_rate — should stabilize after failover
2. Check authorization_rate — should recover with secondary acquirers
3. Verify no PaymentFailedAllRoutes events (indicates all acquirers down)

## Escalation
- **Primary**: On-call SRE
- **Secondary**: Engineering lead (if >30 minutes)
- **Business**: Product owner (if >1 hour, operator notification required)
```

**RUNBOOK-EX-002**: Example runbook — AI Model Degradation:

```markdown
# Runbook: AI Assistant Quality Degradation

## Detection
- **Alert name**: ai_model_quality_drop
- **Symptoms**: hourly quality sampling shows accuracy drop below threshold
- **Metrics**: ai_answer_accuracy < 80% of baseline for 1 hour

## Impact
- **Affected services**: ai-assistant-service, ai-gateway
- **User impact**: AI Assistant may provide incorrect answers
- **Revenue impact**: No direct revenue impact; operational efficiency impact

## Triage Steps
1. Check Ollama inference pool health
2. Review recent model version changes (MODEL-PIN-002)
3. Check retrieval quality metrics (AIMON-007)
4. Review recent data ingestion for anomalies

## Mitigation Steps
1. If model version changed: rollback to previous version (MODEL-ROLLBACK-001)
2. If retrieval degraded: trigger re-embedding (JOB-005)
3. If Ollama issue: restart inference pool
4. If persistent: degrade AI Assistant to raw-data mode (AIGW-005)

## Validation Steps
1. Monitor ai_answer_accuracy — should recover within 1 hour
2. Check retrieval quality metrics — citation_hit_rate should recover
3. Verify AI Assistant responses in dashboard
```

### 13.3 SLSA Level Targets

**SLSA-001**: Build pipeline targets SLSA Level 2:
- Build as code (CI/CD pipeline, not manual)
- Authenticated provenance (signed build attestations)
- Hermetic builds (dependencies pinned, no network access during build)

**SLSA-002**: Container image build process includes SLSA Provenance attestation alongside cosign signature. Provenance is stored as a build artifact and attached to the image.

**SLSA-003**: SBOM is signed alongside the image and stored in the same registry. Consumers can verify SBOM integrity independently.

### 13.4 Chaos Engineering Enhancements

**CHAOS-004**: Additional chaos scenarios:
- **NATS JetStream store corruption**: Simulate corrupted event store; validate recovery procedures (Part 4 RECOVERY-001/002)
- **Outbox relay failure**: Kill the outbox relay process; validate that events are not lost (outbox table retains unpublished entries)
- **Postgres connection pool exhaustion**: Exhaust connection pool; validate that new requests fail fast with appropriate error (not timeout)
- **OpenSearch index corruption**: Corrupt an OpenSearch index; validate that AI Assistant degrades gracefully to raw-data mode

**CHAOS-005**: Chaos engineering findings feed back into:
- Circuit breaker thresholds (Part 3 CB-001)
- Retry configurations (Part 3 RETRY-001)
- Graceful degradation modes (Part 4 AIGW-005)
- Saga timeout values (Part 3 SAGA-004)

### 13.5 Expand-Contract Migration Enhancements

**MIG-004**: Migration validation steps:
1. Migration tested against a staging database clone (MIG-MKCK-002.2)
2. Migration executed during a maintenance window (for breaking changes) or online (for backward-compatible changes)
3. Post-migration verification: automated health checks + manual spot-check
4. Rollback procedure documented and tested before execution

**MIG-005**: Migration history tracking in `migration_history` table (MIG-MKCK-003) includes:
- Migration version number
- Status (pending → applied → verified → rolled_back)
- Maker (who created)
- Checker(s) (who approved)
- Applied at timestamp
- Rollback available (boolean)
- Duration (how long the migration took)

### 13.6 Capacity Planning for NATS JetStream

**NATS-CAP-001**: NATS stream sizing parameters:
- Max stream size: configurable per stream (default: 10GB per bounded context)
- Max message size: 1MB (matching APISEC-001)
- Retention policy: time-based (configurable, default: 7 days) + size-based (max stream size)
- Consumer throughput target: 10,000 events/second per consumer group

**NATS-CAP-002**: Sizing formula: `stream_size = peak_tps × avg_event_size × retention_window_seconds × safety_factor(1.5)`

**NATS-CAP-003**: Consumer lag monitoring: when lag exceeds 10,000 events for >5 minutes, alert to operations. When lag exceeds 100,000 events, consumer enters backpressure mode (Part 4 BACKPRESSURE-002).

### 13.7 Disaster Recovery Enhancements

**DR-005**: DR drill schedule: quarterly restoration from backups into an isolated environment, with application-level correctness validation (not just "the restore command succeeded"). A backup that has never been test-restored is not a verified backup.

**DR-006**: Cross-region replication (Phase 2): active-passive with UAE primary, secondary in another GCC country. DNS failover via health checks + weighted routing.

**DR-007**: RTO/RPO targets (minimum for GA):
- `orchestration-service`: RPO=0, RTO<5 minutes
- `connector-gateway`: RPO=0, RTO<5 minutes
- `iam-service`: RPO=0, RTO<5 minutes
- `reconciliation-service`: RPO<1 hour, RTO<4 hours
- `analytics-service`: RPO<4 hours, RTO<24 hours
- All financial data: RPO=0 (no data loss tolerance for committed events)

### 13.8 Incident Response Tabletop Exercises

**IR-TTX-001**: Quarterly tabletop exercises covering:
- Acquirer outage scenario
- Database failover scenario
- Data breach scenario
- AI model compromise scenario
- Supply chain compromise scenario

**IR-TTX-002**: Annual incident response training for all on-call engineers, covering: detection, triage, mitigation, communication, and post-incident review.

**IR-TTX-003**: Post-incident review for all SEV-1 and SEV-2 incidents (not just "within 48 hours" but with defined remediation tracking and follow-up).

---

## 14. Gap Analysis Additions — Round 2

### 14.1 Infrastructure Degraded Mode Behaviors

**REDIS-DEGRADED-001**: When Redis is unavailable, the API Gateway switches to a degraded rate-limiting mode: local in-memory rate limiter with per-replica counters (approximately 2× normal limit). Idempotency checks bypass Redis and go directly to the event store. Permission checks fall back to Postgres. All degraded-mode operations emit a `RedisDegradedMode` alert.

**NATS-DEGRADED-001**: When NATS is unavailable for >5 minutes, the outbox relay enters degraded mode: continues appending to outbox table (events durably stored), raises critical alert. The outbox table has a hard maximum size (configurable, default: 1M rows). When exceeded, oldest unpublished entries archived to cold storage (MinIO).

**CLICKHOUSE-DEGRADED-001**: When ClickHouse is unavailable, dashboard/report endpoints return last-cached query results with `as_of` timestamp reflecting last successful query, plus `X-Data-Stale: true` header. If staleness exceeds configurable threshold (default: 1 hour), return HTTP 503 with `Retry-After: 60`. Payment processing is unaffected.

**OPENSEARCH-DEGRADED-001**: When OpenSearch is unavailable, the AI Assistant degrades to structured-only mode: can answer direct lookup questions (via Postgres direct query path) but returns "AI Assistant retrieval temporarily unavailable" for questions requiring semantic search.

**MINIO-DEGRADED-001**: When MinIO is unavailable, settlement file ingestion queues files in a local staging directory (on connector-gateway pod's writable volume, max 1GB). Files moved to MinIO when it recovers. Payment processing is unaffected.

### 14.2 Property-Based Testing for Payment State Machine

**PROP-TEST-001**: Property-based test suite for `PaymentIntent` state machine using `proptest`:
- Generate random sequences of valid commands
- Assert invariant preservation after each command
- Specific properties: "no sequence produces `captured_amount > authorized_amount`", "no sequence produces two simultaneous `Authorized` routing attempts", "every `Refunded` intent was previously `Captured`", "a `Voided` intent can never transition to `Captured`"
- Run on every CI build for `orchestration-service`

### 14.3 Payment-Flow-Specific Chaos Scenarios

**CHAOS-PAY-001**: Payment-flow-specific chaos scenarios:
- **Event loss simulation**: Kill outbox relay mid-transaction; verify events eventually published (outbox table retains them)
- **Response loss simulation**: Mock acquirer returning success but dropping response; verify status-check detects actual state
- **Concurrent mutation**: Send simultaneous capture + void requests; verify optimistic concurrency rejects one
- **Double authorization prevention**: Send concurrent authorize requests for same PaymentIntent; verify only one succeeds

### 14.4 Contract Testing

**CONTRACT-001**: Contract test suite (using `prost-build` generated stubs + mock acquirer) validating the gRPC contract between `orchestration-service` and `connector-gateway` at the protobuf level. Validates: valid `AuthorizeRequest`/`AuthorizeResponse` messages, normalized `DeclineReason` value matching, error handling behavior (timeout, circuit open). Run on every CI build.

### 14.5 Concurrent Payment Load Test Scenarios

**LT-CONC-001**: Load test scenarios for concurrent payment operations:
- **Same-card concurrency**: 10 simultaneous checkout attempts with same payment method token → verify only one succeeds
- **Same-intent concurrency**: 5 simultaneous capture requests on same PaymentIntent → verify only one succeeds, others get optimistic concurrency error
- **Refund race**: 3 simultaneous refund requests totaling more than captured amount → verify only first succeeds

### 14.6 Schema Migration Rollback Classification

**MIG-006**: Migration classification table for rollback guidance:
- **Additive-only** (new columns, new tables): safe rollback by code revert
- **Column rename**: requires expand-contract with alias
- **Column drop**: two-phase (deploy code to stop reading → deploy migration to drop)
- **Data backfill**: must be idempotent and re-runnable

### 14.7 Invoice Overdue Transition Reliability

**INV-OVERDUE-001**: JOB-004 uses leader election (LEADER-001) and processes invoices in deterministic order (by `due_date` ASC). Idempotency: transitioning an already-`Overdue` invoice is a no-op.

**INV-OVERDUE-002**: Safety-net sweep checks for invoices more than 3 days past due still in `Sent` status and alerts operations, catching any JOB-004 failure.

### 14.8 AI Output Rate Limiting Per Query Type

**AI-RATE-001**: Per-query-type rate limits: (1) Detailed record queries: max 10 per minute per user, (2) Aggregate/statistical queries: max 30 per minute per user, (3) Document/PDF queries: max 5 per minute per user.

**AI-RATE-002**: Query deduplication: if same query (or semantically equivalent) issued >3 times within 5 minutes, return cached result and log pattern for review.

### 14.9 AI Data Exfiltration Prevention

**AI-EXFIL-001**: Per-query output volume limits: (1) Maximum records in single AI response: configurable, default 100, (2) Maximum date range for aggregate queries: configurable, default 30 days, (3) Queries matching >1000 records require explicit confirmation, (4) Bulk raw record export via AI is prohibited — summaries and statistics only.

**AI-EXFIL-002**: Log all AI queries and result volume; flag queries consistently returning large result sets for review.

### 14.10 Local Development Environment

**DEV-ENV-001**: Local development specification: (1) `docker-compose.yml` for all infrastructure dependencies (Postgres, Redis, NATS, MinIO, ClickHouse, OpenSearch, Ollama), (2) seed data scripts, (3) local feature flag configuration, (4) mock acquirer connector for local testing, (5) AI mock mode (canned responses without GPU).

**DEV-ENV-002**: First-task walkthrough: add a new decline code to the normalization table (Part 7 §3.2) — exercises the full development cycle from trait implementation to conformance test to deployment.

### 14.11 Communication Templates

**IR-COMM-004**: Communication templates for: (1) planned maintenance (7-day advance notice), (2) unplanned degradation (initial within 15min for SEV-1, updates every 30min), (3) full outage (initial within 15min, updates every 15min), (4) post-incident summary (within 48h for SEV-1/2).

### 14.12 Outbox Relay Manual Publish

**RUNBOOK-EX-004**: "Outbox Relay Manual Publish" runbook with SQL query to extract unpublished outbox entries and NATS CLI command to publish them. Validation: after manual publish, check downstream projections are current.

### 14.13 Complete Runbook Priority List

**RUNBOOK-PRIORITY-001**: Runbooks to write before GA, in priority order: (1) data breach response, (2) secret compromise, (3) database complete cluster loss, (4) NATS partition/loss, (5) acquirer outage, (6) AI model degradation, (7) GPU cluster failure, (8) outbox relay failure, (9) Redis failure, (10) MinIO failure.

### 14.14 NFR-AVAIL-001: Availability Target

**NFR-AVAIL-001**: Platform availability target: [99.9% / 99.99%] measured monthly, excluding scheduled maintenance windows. Target set based on pilot merchant's requirements. Measurement: synthetic transaction monitoring from external probe.

### 14.15 NFR-DUR-001: Data Durability Guarantee

**NFR-DUR-001**: All committed financial events (PaymentIntent state transitions, settlement records, ledger entries) must have zero data loss under single-component failure. Durability guarantee: 100% of acknowledged writes are recoverable from persistent storage.

### 14.16 NFR-PERF-001: Minimum TPS Target

**NFR-PERF-001**: Platform must sustain at least [X] payment authorizations per second at p99 latency < [Y]ms under realistic load (3 acquirers, 5% failover rate, 10% 3DS step-up). Values set before M8 based on pilot merchant's expected volume.

### 14.17 Capacity Planning Projections

**CAP-001**: Volume projections (sample): 10,000 transactions/day at initial launch, 100,000/day at 1 year, 500,000/day at 3 years. Storage requirements calculated for each data store at each milestone. Postgres WAL volume per day estimated for continuous archiving cost.

**CAP-002**: AI inference cost projection: monthly GPU cost at 1,000 / 10,000 / 100,000 queries/day. Cost threshold for AI circuit breaker triggering defined.

---

## 15. Open Items Carried Forward

- **OQ-026**: Run the PERF-001 benchmarking spike against provisioned staging infrastructure.
- **OQ-027**: Set specific RPO/RTO numeric targets (§8 DR-001) jointly with Product/Compliance.
- **OQ-028**: Finalize CI/CD blocking-gate stringency for production deploy manual approval.
- **OQ-050**: Finalize canary deployment thresholds (§7.3 CANARY-001) — error-rate and latency thresholds need to be tuned against real production baseline metrics after pilot launch.
- **OQ-051**: Confirm chaos engineering tooling choice (§8.2 CHAOS-001) — Litmus Chaos vs. custom scripts vs. a managed chaos platform — against operational maturity and budget.
- **OQ-052**: Finalize expand-contract migration tooling (§7.4 MIG-002) — `refinery` vs. `sqlx migrate` vs. another migration framework — compatible with the async Rust stack.

---

*End of Part 11. Proceed to Part 12: Appendices & Implementation Roadmap.*
