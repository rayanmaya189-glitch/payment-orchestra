# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 8 of 12:** Identity, Security & Compliance
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 8 of 12 — Identity, Security & Compliance |
| Depends On | Part 1 §6 (no-custody posture), Part 3 (BC-02 IAM, BC-03 Compliance), Part 4 (SVC-02 `iam-service`, mesh mTLS) |
| Feeds Into | Part 9 (encrypted schema fields, audit tables), Part 11 (security testing, DR/backup as part of operational NFRs) |
| Scope | RBAC/ABAC model, authentication, secrets management, encryption, audit framework, PCI-scope minimization, UAE regulatory compliance mapping, AI-specific security controls, incident response posture. |

---

## 1. Authentication

### 1.1 Human Users (Dashboard)

- **AUTH-001**: Email/password with mandatory MFA (TOTP or WebAuthn) for any role above read-only, enforced at `iam-service` (SVC-02).
- **AUTH-002**: Session tokens are short-lived JWTs (access token: 15-minute lifetime) plus a longer-lived refresh token (7-day lifetime, single-use with rotation), both principal-scoped; refresh tokens are revocable (stored server-side reference, not purely stateless) so a compromised session can be invalidated immediately — a pure stateless-refresh design was rejected specifically because it cannot be revoked before natural expiry, which is unacceptable for a financial-operations product.
- **AUTH-003**: Password policy: minimum 12 characters, uppercase + lowercase + digit + special character, checked against HaveIBeenPwned breach database on creation/change. MFA enrollment mandatory for Admin and Finance Operator roles within 24 hours of first login.
- **AUTH-007**: Account lockout: 5 failed login attempts triggers a 15-minute lockout; 10 failed attempts triggers a 1-hour lockout; 20 failed attempts triggers account suspension requiring Admin intervention. Lockout state is tracked per principal in Redis with TTL.
- **AUTH-008**: Session timeout: inactive sessions expire after 30 minutes for Admin/Finance roles, 60 minutes for Developer/Read-Only roles. Active sessions are revalidated on every request against the server-side session store.
- **AUTH-009**: Credential stuffing protection: login endpoint rate-limited to 10 attempts per IP per minute (separate from general API rate limits). Breached-password checking via k-Anonymity API (HaveIBeenPwned) on password creation and change.

### 1.2 Machine/API Clients (Merchant Server-to-Server)

- **AUTH-004**: API key + secret pairs, scoped to specific permission sets (never a single all-powerful key), rotatable without downtime (dual-active-key overlap period: 24 hours). Keys have configurable expiration (default: 90 days, mandatory rotation).
- **AUTH-005**: All API keys are hashed at rest using Argon2id (never stored recoverable-plaintext, even encrypted) — verification is via hash comparison, following the same discipline as password storage.
- **AUTH-010**: API key scope validation happens at the API Gateway (SVC-17) on every request, before the request reaches any domain service. Scope checks are cached in Redis (5-minute TTL) but invalidated immediately on key rotation or revocation.

### 1.3 Service-to-Service (Internal Mesh)

- **AUTH-006**: mTLS via the service mesh (Part 4 §8) provides service identity; `iam-service`'s `ValidatePermission` call additionally carries the propagated actor context (Part 4 §7) so that internal service identity (mesh certificate) and business-actor identity (the human/API-key principal on whose behalf the call is made) are both verifiable and distinct.

---

## 2. Authorization — RBAC + ABAC

### 2.1 RBAC (Role-Based, Coarse-Grained)

Default roles (extensible per tenant, Part 2 UC-001 step 5):

| Role | Typical Permissions |
|---|---|
| Admin | Full tenant configuration, acquirer connection, routing policy, user management |
| Finance Operator | Reconciliation, refunds/voids, invoice/subscription management, reporting — no acquirer credential management |
| Developer | API key management, sandbox access, webhook configuration — no live refund/void authority by default |
| Read-Only | Dashboard/report viewing, AI Assistant Q&A — no mutating actions |
| Compliance Reviewer (internal, platform-operator side) | KYB case review (ACT-06) — scoped to compliance-service only |

### 2.2 ABAC (Attribute-Based, Fine-Grained Overlay)

- **ABAC-001**: Amount-threshold conditions — e.g., a Finance Operator role may be permitted to approve reconciliation-exception resolutions or refunds only up to a configured amount; above threshold requires a second approver (dual-control) — directly resolves OQ-006 (Part 2): yes, a secondary approver is required above a tenant-configurable threshold, enforced at the command-validation layer in the relevant aggregate's command handler (Part 3/5), not merely a UI-level restriction.
- **ABAC-002**: Time/context conditions — e.g., acquirer credential changes may require step-up re-authentication (fresh MFA challenge) regardless of existing session validity, given the sensitivity of that action (BR-010-2 adjacent).
- **ABAC-003**: AI Assistant data-scope conditions — a Read-Only role can query the Assistant but the Assistant's retrieval (Part 6 §3.2) is itself still bounded by the same tenant/role data-access rules as any other read path (e.g., if a future fine-grained role restricts a user to a single acquirer's data, the Assistant's retrieval must respect that same restriction, not have implicit broader access).

### 2.3 Permission Denial Auditing

- **AUTHZ-001**: Every denied authorization check is itself an audited event (`PermissionDenied`, Part 3 §5.2) — repeated denials for a given principal are a security signal (potential privilege-escalation probing or a misconfigured integration) surfaced to the operational alert channel, mirroring the AI guardrail escalation pattern (Part 6 §5.3).

---

## 3. Secrets Management

- **SEC-001**: Acquirer credentials, KYB-partner API keys, and any other third-party secrets are stored using envelope encryption: a per-service Data Encryption Key (DEK) encrypts the secret; the DEK itself is encrypted by a platform-wide Key Encryption Key (KEK) held in a dedicated KMS component (HashiCorp Vault or cloud-native KMS with HSM backing), never co-located with application data (Part 9 elaborates the exact storage split).
- **SEC-002**: Application service credentials (database passwords, internal service tokens) are managed via the deployment platform's native secrets mechanism (Kubernetes Secrets backed by an external secrets manager, per Part 11) — never committed to source control or baked into container images, enforced by CI/CD scanning (Part 11).
- **SEC-003**: Key rotation is supported for both the KEK (platform-wide, scheduled rotation) and per-service DEKs (rotatable independently, e.g., on suspected compromise of a single service's secrets, without requiring a platform-wide re-encryption event).
- **SEC-004**: Key management procedures: KEK ceremonies require dual control (two authorized personnel present), are logged to an immutable audit trail, and KEK material never leaves the HSM boundary. DEK rotation is automated but KEK rotation follows a documented procedure with change management approval.
- **SEC-005**: No secrets in environment variables or command-line arguments. All secrets are mounted via the secrets store (Kubernetes Secrets or Vault) as volumes or via CSI driver — environment variables are rejected by policy because they appear in `/proc/<pid>/environ` and are visible to other processes.

---

## 4. Encryption

### 4.1 In Transit

- **ENC-001**: TLS 1.3 required for all external traffic (terminated at API Gateway, Part 4 §2); TLS 1.2 permitted only as fallback for legacy acquirer integrations that don't support TLS 1.3, with a deprecation timeline. mTLS for all internal service-to-service traffic (Part 4 §8).
- **ENC-002**: Webhook endpoints (inbound from acquirers) require TLS and, per-connector, signature verification (Part 7 §1.2 `verify_webhook_signature`) as a second layer beyond transport security.
- **ENC-006**: Approved cipher suites for TLS 1.3: `TLS_AES_256_GCM_SHA384`, `TLS_CHACHA20_POLY1305_SHA256`, `TLS_AES_128_GCM_SHA256`. Approved cipher suites for TLS 1.2 (fallback only): `TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384`, `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`. All other cipher suites are prohibited.

### 4.2 At Rest

- **ENC-003**: Database-level encryption at rest (Postgres, ClickHouse, OpenSearch, Redis persistence where enabled) via the underlying storage/volume encryption, plus field-level envelope encryption (§3) for the specific categories of especially sensitive data: acquirer credentials, KYB evidence file references, and any tokenized-but-still-sensitive payment method references.
- **ENC-004**: MinIO object storage (documents, exported reports) is encrypted at rest with per-object or per-bucket keys managed through the same KMS component as §3.
- **ENC-007**: All encryption keys (DEK, KEK) are minimum 256-bit (AES-256). RSA keys (where used for JWT signing) are minimum 2048-bit, with 4096-bit recommended.

### 4.3 PCI-DSS Scope Minimization

- **ENC-005**: The platform never stores raw PAN (card numbers) — checkout flows tokenize directly with the acquirer/PSP or via a PCI-compliant tokenization provider, and only acquirer-issued tokens are stored (BR-031-1, Part 2). This is a deliberate architectural choice to keep the platform's own PCI-DSS scope as SAQ-A-adjacent (transmission/redirect only) rather than a full cardholder-data-environment scope — final PCI scoping determination requires a qualified assessor's review, which is a compliance-process activity outside this SRS's engineering scope, but the architecture is designed specifically to make that lighter scope achievable.
- **ENC-008**: PAN flow diagram: Card data enters the system ONLY through the client-side SDK → acquirer's tokenization endpoint. At no point does raw PAN traverse the platform's backend servers, databases, logs, or AI pipeline. This must be documented in a formal PCI-DSS network diagram for QSA review.

### 4.4 Data Classification

- **ENC-009**: All data is classified into four tiers with corresponding protection requirements:

| Classification | Examples | Encryption | Access | Logging |
|---|---|---|---|---|
| **Restricted** | Acquirer API keys, KEK/DEK material, raw PAN (never stored) | AES-256 envelope encryption, HSM-backed KEK | Admin only, step-up MFA | Never log values, log access only |
| **Confidential** | KYB evidence documents, payment method tokens, settlement details | AES-256 envelope encryption | RBAC-scoped, need-to-know | Log access with actor, no payload logging |
| **Internal** | Routing policies, operator config, invoice details | Volume-level encryption | RBAC-scoped | Log mutations only |
| **Public** | Payment link URLs, published API docs | TLS in transit | Unauthenticated for payment links | No logging required |

---

## 5. Audit Framework

### 5.1 Two-Tier Audit Model (Restated and Detailed from Part 3 PRIN-05)

| Tier | Applies To | Mechanism |
|---|---|---|
| Tier 1 — Event-sourced audit (source of truth) | BC-05, BC-08, BC-09, BC-10 | The domain event stream itself (Part 3 §4 envelope: actor, timestamp, causation/correlation IDs) |
| Tier 2 — Command/action audit log | BC-01, BC-02, BC-03, BC-13, BC-14, plus all AI Assistant interactions (Part 6 §3.2, §5) | Append-only audit table per service: actor, action, before/after state snapshot (where meaningful), timestamp, source IP/session |

### 5.2 Retention

- **AUD-001**: Money-movement-relevant audit data (Tier 1) is retained per UAE regulatory expectations for financial records — a specific retention period (commonly multi-year for financial transaction records under UAE Central Bank and AML/CFT expectations) must be confirmed with legal counsel (ties to Part 1 ASSUMP-001/OQ-001) and configured as a platform-wide minimum retention floor that cannot be shortened by any tenant-level data-deletion request (i.e., "right to erasure"-style requests under UAE PDPL, where applicable, must be reconciled against financial record-keeping obligations — a legal/compliance decision, not a purely technical one).
- **AUD-002**: Tier 2 audit logs follow the same retention floor as a matter of consistency and operational simplicity, absent a specific reason to diverge per data category.

### 5.3 Immutability

- **AUD-003**: Audit data (both tiers) is write-once/append-only at the storage layer — no service, including administrative tooling, has an update/delete code path against audit records; corrections are represented as new compensating entries referencing the original, never in-place edits (this mirrors event-sourcing discipline even for the Tier 2 non-event-sourced contexts).

---

## 6. UAE Regulatory & Data Residency Compliance Mapping

*(This section maps architectural controls to regulatory expectation categories; it is not itself a legal opinion — final compliance posture requires UAE legal counsel sign-off per Part 1 §6.4/ASSUMP-001.)*

| Regulatory Expectation Category | Architectural Control |
|---|---|
| UAE Central Bank Retail Payment Services Regulation — licensable-activity boundary | Part 1 §6 no-custody architecture; Part 3 structural absence of platform-owned-balance aggregates |
| AML/CFT record-keeping | §5 audit framework (Tier 1/2), §5.2 retention floor, §11.1 AML transaction monitoring |
| AML/CFT suspicious activity reporting | §11.1 AML-002/003, SAR generation capability |
| KYC/KYB evidence handling | Part 2 UC-002, Part 3 BC-03, this Part §3/§4 (evidence encryption) |
| Data residency (UAE PDPL and sector expectations) | Part 1 ASSUMP-004; full stack deployed within UAE-region infrastructure |
| Consumer data protection (PDPL) | RBAC/ABAC scoping (§2), encryption (§4), §14 DEST-003 data subject request handling |
| Card scheme compliance (PCI-DSS) | §4.3 scope minimization, §7.5 SECTEST-005, §12.4 error handling |
| Fraud monitoring requirements | §11.2 FRAUD-001 through FRAUD-003, risk scoring service |
| Regulatory reporting | §11.3 REG-001/002, automated report generation |

---

## 7. Security Testing Requirements

- **SECTEST-001**: A dedicated security isolation test suite must run against every release: automated attempts to access unauthorized data via every API surface, using role-violation test fixtures to catch authorization bypass (e.g., a Read-Only user attempting mutating operations, a Developer role attempting acquirer credential changes).
- **SECTEST-002**: Periodic third-party penetration testing (quarterly) covering the API Gateway, AI Gateway, and connector webhook endpoints specifically, given their exposure to untrusted/external input. Findings with CVSS >= 7.0 must be remediated within 30 days.
- **SECTEST-003**: Static analysis (Clippy with security lints, `cargo-audit`, `cargo-deny`) and dependency vulnerability scanning integrated into CI/CD (Part 11) — Rust's memory-safety guarantees reduce but do not eliminate the need for this (logic-level vulnerabilities, e.g., authorization bypass, are not caught by memory safety).
- **SECTEST-004**: Row-Level Security (RLS) policies enforced at the Postgres database layer for all tables containing operator data — provides defense-in-depth against application-layer authorization bypass. RLS policies are tested as part of SECTEST-001.
- **SECTEST-005**: Annual PCI-DSS assessment (SAQ-A or SAQ-A-EP depending on final scoping) by a Qualified Security Assessor (QSA). Internal quarterly vulnerability scans by an Approved Scanning Vendor (ASV).

### 7.1 OWASP-Specific Security Controls

| OWASP Category | Control | Implementation |
|---|---|---|
| A01 Broken Access Control | Method-level RBAC, RLS policies, CORS policy | §2 RBAC/ABAC, SECTEST-004, §10 CORS |
| A02 Cryptographic Failures | TLS 1.3 mandate, AES-256, HSM-backed KEK | §4 ENC-001/006/007 |
| A03 Injection | Parameterized queries, input sanitization | Rust type system + sqlx, §10 input validation |
| A04 Insecure Design | Abuse case modeling, abuse rate limits | §10 abuse prevention |
| A05 Security Misconfiguration | Hardened containers, security headers, NetworkPolicies | §10 security headers, §10 container hardening |
| A06 Vulnerable Components | SBOM, dependency pinning, CVE SLA | §10 supply chain |
| A07 Auth Failures | Account lockout, session management, MFA | §1 AUTH-007/008/009 |
| A08 Data Integrity | Image signing, SLSA, migration signing | §10 CI/CD integrity |
| A09 Logging Failures | SIEM integration, log integrity, alerting | §10 logging security |
| A10 SSRF | URL validation, DNS rebinding protection | §10 SSRF prevention |

### 7.2 Security Headers

- **HDR-001**: API Gateway (SVC-17) and payment link hosted pages (SVC-07) must include the following HTTP security headers:
  - `Strict-Transport-Security: max-age=31536000; includeSubDomains; preload`
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'`
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()`
  - `X-XSS-Protection: 0` (disabled — modern browsers handle CSP better)

- **HDR-002**: CORS policy: API Gateway only allows origins explicitly whitelisted in operator configuration. Default: no cross-origin requests allowed. Payment link pages (public) have a more permissive CSP but still restrict frame-ancestors.

### 7.3 Container Hardening

- **HARD-001**: All container images must:
  - Run as non-root user (UID >= 1000)
  - Use distroless or scratch base images (no shell, no package manager)
  - Have a read-only root filesystem (writable volumes only for `/tmp` and data directories)
  - Have no capabilities granted beyond `NET_BIND_SERVICE` (port 80/443 binding)
  - Pass Trivy/Grype vulnerability scanning with zero critical/high findings

- **HARD-002**: Kubernetes NetworkPolicies: default-deny all ingress/egress at the namespace level; explicit allow-lists per service pair (e.g., `api-gateway` → `orchestration-service` on port 50051, `orchestration-service` → `connector-gateway` on port 50052).

### 7.4 SSRF Prevention

- **SSRF-001**: All outbound HTTP requests (webhook delivery, acquirer API calls, document URL fetching) must validate the target URL against a deny-list of private/reserved IP ranges (RFC 1918, RFC 3927, RFC 4193, link-local) before making the request. DNS resolution is performed and the resolved IP is checked, not just the hostname.
- **SSRF-002**: Webhook URL registration (Part 10 WEBHOOK-001) validates that the URL resolves to a public IP (not internal/private ranges) and that the URL scheme is HTTPS only.

### 7.5 Logging Security

- **LOGSEC-001**: Audit logs and application logs are stored in append-only storage (e.g., S3 with Object Lock, or a dedicated log store with immutability guarantees). Log integrity is verified via cryptographic hash chains (each log entry includes a hash of the previous entry).
- **LOGSEC-002**: Sensitive data (credentials, PAN, full card tokens, MFA secrets, raw API keys) is NEVER logged. Log statements are reviewed for this specifically as part of code review checklist. A log-scanning CI step detects potential sensitive data in log output.
- **LOGSEC-003**: SIEM integration: security-relevant events (failed logins, permission denials, suspicious patterns, guardrail violations) are forwarded to a SIEM platform with automated alerting rules:
  - 5+ failed logins in 5 minutes → alert
  - 10+ permission denials in 10 minutes for same principal → alert
  - AI guardrail violation (prompt injection detected) → alert
  - Acquirer credential access outside business hours → alert
  - Any attempt to access Restricted-classification data → alert

### 7.6 Supply Chain Security

- **SUPPLY-001**: A Software Bill of Materials (SBOM) in SPDX or CycloneDX format is generated for every release and stored as a build artifact.
- **SUPPLY-002**: All dependencies are pinned in `Cargo.lock` (committed to source control). Dependency updates are managed via automated tooling (e.g., Dependabot/Renovate) with security review required for major version bumps.
- **SUPPLY-003**: Container images are signed using cosign/sigstore. Image provenance is verified at deploy time (Kubernetes admission controller rejects unsigned images).
- **SUPPLY-004**: Critical/high CVE response SLA: patches within 24 hours for critical (CVSS >= 9.0), 7 days for high (CVSS >= 7.0), 30 days for medium (CVSS >= 4.0).

### 7.7 Privileged Access Management

- **PAM-001**: Production database access requires just-in-time (JIT) access approval — engineers request time-bounded access (maximum 4 hours) through an approval workflow; access is automatically revoked after expiry.
- **PAM-002**: All production SSH sessions and database queries are recorded (session recording) and retained for 90 days for audit review.
- **PAM-003**: No standing production access for any engineer. All production access is via a bastion host or ephemeral access mechanism with audit logging.

---

## 8. AI-Specific Security Controls (Cross-Reference to Part 6)

This Part is the authoritative home for the *security/compliance framing* of controls whose *mechanism* is detailed in Part 6:

- **AISEC-001**: Retrieval index access control is a security control, validated by SECTEST-001's security suite extended to include AI Assistant query paths specifically.
- **AISEC-002**: The AI guardrail audit log (Part 4 §3.2 AIGW-004, Part 6 §3.2 step 6) is a Tier 2 audit mechanism (§5.1 above) and subject to the same retention floor (§5.2).
- **AISEC-003**: Prompt-injection screening (Part 6 §5.1 GRD-IN-001) is treated as a security control subject to the same penetration-testing cadence as other externally-exposed input surfaces (§7 SECTEST-002).

---

## 9. Incident Response Posture (Preview)

- **IR-001**: A documented incident response runbook (finalized in Part 11 operationally, but its existence is a compliance requirement recorded here) must cover: suspected credential compromise (platform-level), suspected data exposure, AI guardrail bypass patterns, and acquirer-side outage handling.
- **IR-002**: Any confirmed data exposure or credential compromise affecting live payment credentials triggers a defined notification process to the affected operator and, where required by UAE regulatory/data-protection obligations, to relevant authorities.

---

## 10. Threat Model (STRIDE-Based)

### 10.1 External Attacker Surface

| Threat | Target | Mitigation |
|---|---|---|
| **Spoofing** | API Gateway endpoints, webhook ingress | TLS 1.3 + JWT/API-key auth (§1), webhook signature verification (Part 7), mTLS for internal traffic (§1.3 AUTH-006), MFA for human users (AUTH-001) |
| **Tampering** | Payment intent amounts, routing policies, event streams | Event-sourcing immutability (PRIN-05), optimistic concurrency (Part 5 CONC-001), idempotency keys, outbox pattern (Part 3 §9.2) |
| **Repudiation** | Configuration changes, money-movement events | Two-tier audit framework (§5), immutable event stream, `PermissionDenied` logging (§2.3 AUTHZ-001), log integrity hash chains (LOGSEC-001) |
| **Information Disclosure** | Unauthorized data access, sensitive data in logs | RBAC/ABAC enforcement (§2), RLS policies (SECTEST-004), encrypted fields (§4), log scrubbing (LOGSEC-002), data classification (ENC-009) |
| **Denial of Service** | Checkout path, AI Assistant, database | Rate limiting (Part 4 GW-003, Part 10 RL-001), account lockout (AUTH-007), circuit breakers (Part 3 §9.3), GPU isolation (Part 4 K8S-002), request size limits (REQ-001) |
| **Elevation of Privilege** | RBAC bypass, ABAC threshold bypass | Permission validation on every request (Part 4 GW-002), ABAC enforcement at aggregate command level (§2.2 ABAC-001), step-up re-auth for sensitive actions (ABAC-002), API key scope validation (AUTH-010) |

### 10.2 Insider Threat

| Threat | Target | Mitigation |
|---|---|---|
| **Compromised admin account** | Operator data/config modification | MFA enforcement (§1.1 AUTH-001), step-up re-auth for acquirer credential changes (ABAC-002), audit trail of all changes (§5), account lockout (AUTH-007) |
| **Malicious operator** | Data exfiltration, configuration tampering | mTLS service identity + propagated actor context (§1.3 AUTH-006), no super-admin bypass, audit log of all admin actions, PAM (§7.7), session recording (PAM-002) |
| **Compromised AI model/inference** | Data exfiltration via model output | AI Gateway guardrails (Part 6 §5), output monitoring (GRD-IN-003), no write-path tool access (AI-P-003), prompt injection defense (GRD-IN-003) |
| **Compromised database credentials** | Direct database access bypassing application | RLS policies (SECTEST-004), encrypted credentials (SEC-001), JIT access (PAM-001), database activity monitoring (DBA-001) |

### 10.3 Supply Chain

| Threat | Target | Mitigation |
|---|---|---|
| **Compromised acquirer connector** | Malicious response injection | ACL pattern (Part 3 §8), normalized response types (Part 7 CONN-001/002), response schema validation before aggregate state mutation, circuit breakers (CB-001) |
| **Compromised Ollama model** | Prompt injection, data exfiltration | Self-hosted inference (AI-P-004), input sandboxing (GRD-IN-003), output monitoring, model integrity verification (hash checking on model load), SUPPLY-003 |
| **Compromised dependency** | Remote code execution, data theft | SBOM (SUPPLY-001), dependency pinning (SUPPLY-002), `cargo audit` + `cargo deny` in pipeline (SECTEST-003), Rust's memory safety reducing attack surface, CVE SLA (SUPPLY-004) |
| **Compromised NATS/stream** | Event injection, event loss | Outbox pattern (Part 3 §9.2) for publish reliability, durable consumers (Part 4 §4.2), event signature verification (future enhancement) |
| **Compromised container image** | Runtime compromise | Image signing (SUPPLY-003), hardened base images (HARD-001), vulnerability scanning (HARD-001), admission controller rejects unsigned images |

### 10.4 Formal Threat Model Process

- **TM-001**: A formal threat model document is maintained per major feature, using the STRIDE methodology with DREAD risk scoring. Threat models are reviewed quarterly and updated when new features are added or when threat intelligence indicates new attack vectors.
- **TM-002**: Red team exercises are conducted semi-annually by an external security firm, targeting the API Gateway, AI Gateway, connector webhook endpoints, and admin interfaces. Findings are tracked to remediation with SLA per CVSS score.

### 10.5 Abuse Case Catalog

| Abuse Case | OWASP | Risk | Mitigation |
|---|---|---|---|
| Brute-force login | A07 | High | AUTH-007 (lockout), AUTH-009 (rate limit) |
| Credential stuffing | A07 | High | AUTH-009 (breach checking), AUTH-007 (lockout) |
| SQL injection via API | A03 | Critical | Parameterized queries (sqlx), RLS (SECTEST-004) |
| SSRF via webhook URL | A10 | High | SSRF-001/002 (URL validation) |
| Prompt injection via AI | A03 | High | GRD-IN-003 (multi-layer defense) |
| Privilege escalation via API | A01 | Critical | Method-level RBAC, RLS (SECTEST-004) |
| Data exfiltration via logs | A09 | High | LOGSEC-002 (log scrubbing), LOGSEC-003 (SIEM) |
| Supply chain compromise | A06 | Critical | SUPPLY-001/002/003/004 |
| Replay attack on webhooks | A08 | Medium | WEBHOOK-REPLAY-001/002/003 (Part 10) |
| Account takeover | A07 | High | AUTH-001 (MFA), AUTH-007 (lockout), AUTH-008 (session timeout) |

---

## 11. AML/CFT and Fraud Monitoring

### 11.1 AML Transaction Monitoring

- **AML-001**: The platform implements rule-based transaction monitoring for suspicious patterns:
  - **Structuring detection**: Multiple transactions just below reporting thresholds within a configurable window
  - **Velocity checks**: Unusual transaction frequency or volume for the operator's historical baseline
  - **Amount anomalies**: Transactions significantly exceeding the operator's average transaction size
  - **Rapid succession**: Multiple authorizations on the same payment method within a short window

- **AML-002**: Alert queue for flagged transactions, reviewed by the Compliance Reviewer role (Part 8 §2.1). Flagged transactions are not blocked automatically (the platform doesn't hold funds) but are surfaced for the operator's compliance team and, where required, reported to UAE Financial Intelligence Unit (FIU).

- **AML-003**: Suspicious Activity Reports (SARs) can be generated from the compliance dashboard and exported in the format required by UAE regulatory authorities. SAR generation and submission are audit-logged.

### 11.2 Fraud Scoring Integration

- **FRAUD-001**: The risk-scoring service (BC-11) provides a real-time risk score for each transaction before authorization, based on:
  - Card BIN analysis (issuing country, card type, fraud history)
  - Transaction velocity (per card, per IP, per device fingerprint)
  - Amount pattern analysis
  - Geographic anomalies (billing/shipping mismatch, IP geolocation)

- **FRAUD-002**: Risk scores are surfaced to the operator via the dashboard and optionally exposed via webhook. High-risk transactions can be flagged for manual review or declined (per operator-configurable thresholds) before authorization is attempted.

- **FRAUD-003**: Fraud analytics dashboard showing: fraud rate by acquirer, chargeback rate trends, declined-transaction patterns, and BIN-level fraud concentration.

### 11.3 Regulatory Reporting

- **REG-001**: The platform generates the following reports required by UAE Central Bank:
  - Monthly transaction volume report (by acquirer, card scheme, currency)
  - Chargeback rate report (per acquirer, per card scheme — scheme compliance thresholds)
  - Suspicious activity summary (aggregated from AML alerts)
  - System availability and incident report (for payment service reliability)

- **REG-002**: Reports are generated automatically on a scheduled basis and stored in the `exported-reports` MinIO bucket (Part 9 §5) with compliance retention. Manual on-demand report generation is also available.

---

## 12. API Security

### 12.1 Request Security

- **REQ-001**: Request size limits enforced at API Gateway: maximum 1MB for standard requests, maximum 10MB for document upload endpoints, maximum 100KB for payment creation. Limits enforced before body parsing to prevent memory exhaustion.
- **REQ-002**: Request timeout: API Gateway enforces a 30-second timeout on all downstream service calls. Checkout-path endpoints (CreatePaymentIntent, AuthorizePaymentIntent) have a tighter 10-second timeout. Acquirer-facing calls have a configurable per-connector timeout (default: 15 seconds for authorize, 30 seconds for settlement polling).
- **REQ-003**: Request correlation: every inbound request generates a UUIDv7-based `request_id` that is propagated through all downstream calls, included in error responses, and correlated with distributed traces (OBS-005).

### 12.2 Input Validation

- **INPUT-001**: All API inputs are validated against their OpenAPI/gRPC schema at the API Gateway before reaching domain services. Schema validation includes:
  - Type validation (string, integer, enum, etc.)
  - Length/range validation (min/max values, string length limits)
  - Format validation (email, UUIDv7, ISO 4217 currency code, ISO 8601 timestamp)
  - Required field validation

- **INPUT-002**: Domain-specific validation happens in the command handler (Part 3 PRIN-01) — the API Gateway handles syntactic validation; the domain layer handles semantic validation (e.g., "amount must be positive," "currency must be supported," "acquirer link must be Active").

### 12.3 CORS Policy

- **CORS-001**: API Gateway CORS policy:
  - `Access-Control-Allow-Origin`: Only explicitly whitelisted origins (operator-configured dashboard domains)
  - `Access-Control-Allow-Methods`: GET, POST, PUT, PATCH, DELETE, OPTIONS
  - `Access-Control-Allow-Headers`: Content-Type, Authorization, X-Api-Key, X-Idempotency-Key, X-Request-ID
  - `Access-Control-Allow-Credentials`: true (for cookie-based auth)
  - `Access-Control-Max-Age`: 86400 (24 hours preflight cache)

### 12.4 Error Handling

- **ERR-001**: API error responses never expose:
  - Stack traces or internal error details
  - Database query errors or connection strings
  - File paths or internal service names
  - Version information that could reveal attack surface

- **ERR-002**: Error codes are stable, documented enums (Part 10 API-005). Generic error messages are returned to callers; detailed error context is logged server-side for debugging.

---

## 13. Key Management Procedures

- **KMP-001**: Key Custody: KEK material is held in an HSM (FIPS 140-2 Level 3 or equivalent). No single individual has access to the complete KEK — split knowledge/dual control is enforced.
- **KMP-002**: Key Rotation Schedule: KEK rotated every 90 days (configurable, SEC-ROT-001). DEKs rotated on-demand (compromise) or annually. API keys rotated every 90 days (configurable, AUTH-004).
- **KMP-003**: Key Destruction: When keys are retired, the old key material is cryptographically destroyed (overwritten in HSM) after ensuring all data encrypted under it has been re-encrypted or archived. Destruction is logged.
- **KMP-004**: Emergency Key Revocation: In case of suspected key compromise, an emergency revocation procedure allows immediate KEK rotation (bypassing normal change management) with mandatory post-incident review within 48 hours.

---

## 14. Data Destruction and Retention

- **DEST-001**: Data destruction follows a formal procedure:
  - Cryptographic erasure: Data encrypted under a DEK is rendered irrecoverable by destroying the DEK (SEC-003/004).
  - Physical destruction: For infrastructure decommissioning, storage media is physically destroyed per NIST SP 800-88 guidelines.
- **DEST-002**: Retention periods per data classification:
  - Restricted: Per regulatory requirement (typically 5+ years for financial records)
  - Confidential: Per regulatory requirement + business need
  - Internal: 2 years minimum, then archival
  - Public: No minimum retention
- **DEST-003**: Data Subject Requests (UAE PDPL): A documented process for handling data subject access/erasure requests, reconciled against financial record retention obligations (OQ-019). Erasure requests are denied for data subject to financial record-keeping requirements, with explanation provided to the requester.

---

## 15. Traceability

| Requirement | Realized By |
|---|---|
| BIZ-040 (immutable audit) | §5 (both tiers), §5.3 AUD-003 |
| BIZ-041 (data residency) | §6 table row 4 |
| BIZ-042 (RBAC/ABAC, elevated-permission audit) | §2, §2.3 |
| BIZ-043 (KYB evidence, not platform decisioning) | §6 table row 3, Part 3 BC-03 cross-reference |
| OQ-006 (Part 2, secondary approver threshold) | §2.2 ABAC-001 — resolved: yes, threshold-based dual control |
| BR-031-1 (Part 2, tokenization not raw PAN) | §4.3 ENC-005 |
| Part 6 AI-P-002/AI-P-003 | §8 AISEC-001/002 |
| Threat model (STRIDE) | §10.1 through §10.3 |
| API key scoping to acquirer links | §11 AUTHZ-002, AUTHZ-003 |
| Secrets rotation automation | §12 SEC-ROT-001 through SEC-ROT-004 |

---

## 16. Open Items Carried Forward

- **OQ-018**: Confirm exact financial-record retention period (§5.2 AUD-001) with UAE legal counsel.
- **OQ-019**: Confirm whether PDPL-style data-subject erasure requests are applicable to platform-processed payment/financial records.
- **OQ-044**: Finalize KEK rotation schedule (§12 SEC-ROT-001, default 90 days) against operational risk assessment — more frequent rotation increases security but adds KMS load.
- **OQ-045**: Confirm whether API key scoping to acquirer links (§11 AUTHZ-002) is required for MVP or deferred to H2 — depends on pilot merchant integration complexity.

---

*End of Part 8. Proceed to Part 9: Database Design.*
