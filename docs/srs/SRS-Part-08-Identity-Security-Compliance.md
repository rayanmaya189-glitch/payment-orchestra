# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

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
| Scope | ABAC model, authentication, secrets management, encryption, audit framework, PCI-scope minimization, UAE regulatory compliance mapping, AI-specific security controls, incident response posture. |

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

- **AUTH-006**: mTLS via the API Gateway (Part 4 §8) provides service identity; internal modules use in-process identity propagation (Part 4 §7); `iam-service`'s `ValidatePermission` call additionally carries the propagated actor context (Part 4 §7) so that internal service identity (mesh certificate) and business-actor identity (the human/API-key principal on whose behalf the call is made) are both verifiable and distinct.

---

## 2. Authorization — ABAC (Attribute-Based Access Control)

### 2.1 Role Definitions

Default roles (extensible per operator, Part 2 UC-001 step 5) serve as attribute groupings for ABAC policy assignment:

| Role | Default Attribute Set | Maker/Checker Eligibility |
|---|---|---|
| Admin | Full operator configuration access, acquirer connection, routing policy, user management | Maker: all operations. Checker: all operations EXCEPT self-approved changes (MKCK-002) |
| Finance Operator | Reconciliation, refunds/voids, invoice/subscription management, reporting — no acquirer credential management; refund amount threshold: configurable (default: 50,000 AED) | Maker: refunds (below threshold), reconciliation. Checker: refunds (below threshold) |
| Developer | API key management, sandbox access, webhook configuration — no live refund/void authority by default | Maker: API key generation (limited). Checker: none (cannot approve changes) |
| Read-Only | Dashboard/report viewing, AI Assistant Q&A — no mutating actions | Neither Maker nor Checker for any operation |
| Compliance Reviewer (internal, platform-operator side) | KYB case review (ACT-06) — scoped to compliance-service only | Maker: KYB approvals, AML alert resolution. Checker: AML alert resolution |

### 2.2 ABAC Policy Rules

- **ABAC-001**: Amount-threshold conditions — e.g., a Finance Operator role may be permitted to approve reconciliation-exception resolutions or refunds only up to a configured amount; above threshold requires a second approver (dual-control) — enforced at the command-validation layer in the relevant aggregate's command handler (Part 3/5), not merely a UI-level restriction. This directly resolves OQ-006 (Part 2): yes, a secondary approver is required above a configurable threshold.
- **ABAC-002**: Time/context conditions — e.g., acquirer credential changes may require step-up re-authentication (fresh MFA challenge) regardless of existing session validity, given the sensitivity of that action (BR-010-2 adjacent).
- **ABAC-003**: AI Assistant data-scope conditions — a Read-Only role can query the Assistant but the Assistant's retrieval (Part 6 §3.2) is itself still bounded by the same attribute-based data-access rules as any other read path.
- **ABAC-004**: Maker/Checker segregation — no principal may serve as both Maker and Checker on the same pending change (MKCK-002, Part 3). The system enforces this at the command layer: `ApprovePendingChange` rejects if `checker_id == maker_id`.
- **ABAC-005**: Checker eligibility is scoped by role attribute — only Admin and Compliance Reviewer roles can serve as Checkers for financial/compliance operations. Developers cannot approve financial changes even if they have Maker access to initiate them.
- **ABAC-006**: Scope conditions — API keys can be scoped to specific acquirer links (Part 8 §11 AUTHZ-002), restricting which acquirers a key can interact with. The scope is evaluated as an attribute condition at the API Gateway on every request.
- **ABAC-007**: IP/context conditions — certain operations (e.g., bulk refund, routing policy export) may be restricted to requests originating from approved IP ranges (e.g., office network IPs for compliance-sensitive operations).
- **ABAC-008**: Rate/quota conditions — AI Assistant usage is bounded by per-operator quotas (Part 4 AIGW-002), enforced as an attribute condition on the operator's usage counter.

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
| **Confidential** | KYB evidence documents, payment method tokens, settlement details | AES-256 envelope encryption | ABAC-scoped, need-to-know | Log access with actor, no payload logging |
| **Internal** | Routing policies, operator config, invoice details | Volume-level encryption | ABAC-scoped | Log mutations only |
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

## 6. Compliance Framework Alignment

### 6.1 SWIFT Customer Security Programme (CSP) Alignment

The SWIFT CSP defines mandatory security controls for payment platform participants. While the platform is not a direct SWIFT participant, aligning with CSP controls demonstrates payment industry security maturity:

| CSP Control | Category | Platform Implementation |
|---|---|---|
| 1.1 Restrict Internet Access | Network Security | NetworkPolicies (Part 11 K8S-005), mTLS (Part 4 §8) |
| 1.2 Restrict Key Management System Access | Access Control | HSM-backed KEK (§3 SEC-001), PAM (§7.7) |
| 1.3 Manage Windows Privileged Account | Privileged Access | PAM (§7.7), JIT access, session recording |
| 1.4 Manage Unix/Linux Privileged Account | Privileged Access | PAM (§7.7), bastion host, no standing access |
| 2.1 Protections of Security Infrastructure | Network Security | Container hardening (Part 11 HARD-001), security headers (§7.2) |
| 2.2 Reduce Attack Surface | Network Security | NetworkPolicies (Part 11 K8S-005), distroless containers (Part 11 HARD-001) |
| 2.3 Secure Configuration | Network Security | CIS benchmark compliance, secure defaults |
| 2.4 Ensure Integrity of Custom Software | Application Security | TDD (Part 11 §1), code review, SAST (Part 11 SECPIPE-001) |
| 2.5 Securely Transfer Sensitive Data | Data Protection | TLS 1.3 (§4 ENC-001), mTLS (Part 4 §8) |
| 3.1 Segregation of Duties | Access Control | ABAC (§2), Maker/Checker (Part 3 §9.2) |
| 3.2 Least Privilege | Access Control | ABAC (§2), API key scoping (§11) |
| 3.3 Physical Security | Physical Security | Cloud provider responsibility (documented) |
| 3.4 System Hardening | System Security | Container hardening (Part 11 HARD-001), CIS benchmarks |

### 6.2 ISO 27001 Annex A Alignment

| Annex A Control | Category | Platform Implementation |
|---|---|---|
| A.5.1 Policies for information security | Organizational | Information Security Policy (Part 11 COMPLDOC-001) |
| A.5.2 Information security roles and responsibilities | Organizational | ABAC roles (§2.1), RACI matrix (Part 1 §8.3) |
| A.5.3 Information security awareness, education and training | Organizational | Security Awareness Training Program (Part 11 COMPLDOC-001) |
| A.6.1 Screening | People | KYB evidence workflow (Part 2 UC-002) |
| A.6.2 Terms and conditions of employment | People | Acceptable Use Policy (Part 11 COMPLDOC-001) |
| A.6.3 Information security awareness, education and training | People | Security Awareness Training Program |
| A.7.1 Physical security perimeters | Physical | Cloud provider responsibility |
| A.7.2 Physical entry | Physical | Cloud provider responsibility |
| A.8.1 User endpoint devices | Technological | Dashboard security headers (§7.2), CSP |
| A.8.2 Privileged access rights | Technological | PAM (§7.7), ABAC (§2) |
| A.8.3 Information access restriction | Technological | ABAC (§2), RLS (SECTEST-004) |
| A.8.4 Access to source code | Technological | Git access controls, code review |
| A.8.5 Secure authentication | Technological | MFA (§1.1 AUTH-001), API keys (§1.2) |
| A.8.6 Capacity management | Technological | Connection pooling (Part 9 §9), rate limiting (Part 10 §5) |
| A.8.7 Protection against malware | Technological | Container scanning (Part 11 SECPIPE-001), dependency scanning |
| A.8.8 Management of technical vulnerabilities | Technological | CVE SLA (§7.6 SUPPLY-004), dependency scanning |
| A.8.9 Configuration management | Technological | Hardened containers (Part 11 HARD-001), K8s NetworkPolicies |

### 6.3 NIST Cybersecurity Framework Alignment

| CSF Function | Category | Platform Implementation |
|---|---|---|
| **Identify** | Asset Management | Data classification (§4.4 ENC-009), SBOM (§7.6 SUPPLY-001) |
| **Identify** | Risk Assessment | Threat model (§10), abuse cases (§10.5) |
| **Protect** | Access Control | ABAC (§2), MFA (§1.1), PAM (§7.7) |
| **Protect** | Data Security | Encryption (§4), tokenization (§4.3), key management (§13) |
| **Protect** | Protective Technology | Container hardening (Part 11), security headers (§7.2) |
| **Detect** | Anomalies and Events | SIEM (§7.5), alerting (LOGSEC-003), audit logs (§5) |
| **Detect** | Security Continuous Monitoring | Pen testing (§7.2), vulnerability scanning (§7.1) |
| **Respond** | Response Planning | Incident response runbook (§9.3), communication plan (§9.2) |
| **Respond** | Communications | Incident communication plan (§9.2) |
| **Respond** | Analysis | Incident post-mortem (§9.2 IR-COMM-003) |
| **Respond** | Mitigation | Incident response runbook playbooks (§9.3) |
| **Recover** | Recovery Planning | DR plan (Part 11 §8), BCP (Part 11 §8.1) |
| **Recover** | Improvements | Post-mortem remediation tracking (§9.2 IR-COMM-003) |
| **Recover** | Communications | Incident communication plan (§9.2) |

### 6.4 UAE Regulatory & Data Residency Compliance Mapping

*(This section maps architectural controls to regulatory expectation categories; it is not itself a legal opinion — final compliance posture requires UAE legal counsel sign-off per Part 1 §6.4/ASSUMP-001.)*

| Regulatory Expectation Category | Architectural Control |
|---|---|
| UAE Central Bank Retail Payment Services Regulation — licensable-activity boundary | Part 1 §6 no-custody architecture; Part 3 structural absence of platform-owned-balance aggregates |
| AML/CFT record-keeping | §5 audit framework (Tier 1/2), §5.2 retention floor, §11.1 AML transaction monitoring |
| AML/CFT suspicious activity reporting | §11.1 AML-002/003, SAR generation capability |
| KYC/KYB evidence handling | Part 2 UC-002, Part 3 BC-03, this Part §3/§4 (evidence encryption) |
| Data residency (UAE PDPL and sector expectations) | Part 1 ASSUMP-004; full stack deployed within UAE-region infrastructure |
| Consumer data protection (PDPL) | ABAC scoping (§2), encryption (§4), §14 DEST-003 data subject request handling |
| Card scheme compliance (PCI-DSS) | §4.3 scope minimization, §7.5 SECTEST-005, §12.4 error handling |
| Fraud monitoring requirements | §11.2 FRAUD-001 through FRAUD-003, risk scoring service |
| Regulatory reporting | §11.3 REG-001/002, automated report generation |

---

## 7. Security Testing Requirements

- **SECTEST-001**: A dedicated security isolation test suite must run against every release: automated attempts to access unauthorized data via every API surface, using role-violation test fixtures to catch authorization bypass (e.g., a Read-Only user attempting mutating operations, a Developer role attempting acquirer credential changes).
- **SECTEST-002**: Periodic third-party penetration testing (quarterly) covering the API Gateway, AI Gateway, and connector webhook endpoints specifically, given their exposure to untrusted/external input. Findings with CVSS >= 7.0 must be remediated within 30 days.

#### 7.2 Penetration Testing Scope

- **PENTEST-SCOPE-001**: Scope boundaries:
  - **In-scope**: API Gateway (REST/gRPC-Web), AI Gateway, connector webhook endpoints, payment link hosted pages, admin dashboard, all domain service APIs accessible through the API Gateway
  - **Out-of-scope**: Internal service-to-service gRPC (protected by mTLS), database direct access, infrastructure layer (cloud provider responsibility), third-party acquirer systems

- **PENTEST-SCOPE-002**: Testing methodology:
  - **Black-box**: External tester with no prior knowledge (simulates external attacker)
  - **Grey-box**: Tester with limited knowledge (API docs, user credentials) (simulates malicious insider with limited access)
  - **White-box**: Full system access (architecture diagrams, source code) (simulates comprehensive security review)
  - Quarterly rotations alternate between black-box and grey-box; white-box is performed annually.

- **PENTEST-SCOPE-003**: Reporting format:
  - Executive summary (business risk)
  - Findings with CVSS score, affected endpoint, reproduction steps
  - Remediation recommendations with priority
  - Timeline for remediation (Critical: 24h, High: 7d, Medium: 30d, Low: 90d)
  - Verification of prior finding remediation
- **SECTEST-003**: Static analysis (Clippy with security lints, `cargo-audit`, `cargo-deny`) and dependency vulnerability scanning integrated into CI/CD (Part 11) — Rust's memory-safety guarantees reduce but do not eliminate the need for this (logic-level vulnerabilities, e.g., authorization bypass, are not caught by memory safety).
- **SECTEST-004**: Row-Level Security (RLS) policies enforced at the Postgres database layer for all tables containing operator data — provides defense-in-depth against application-layer authorization bypass. RLS policies are tested as part of SECTEST-001.
- **SECTEST-005**: Annual PCI-DSS assessment (SAQ-A or SAQ-A-EP depending on final scoping) by a Qualified Security Assessor (QSA). Internal quarterly vulnerability scans by an Approved Scanning Vendor (ASV).

### 7.1 OWASP-Specific Security Controls

| OWASP Category | Control | Implementation |
|---|---|---|
| A01 Broken Access Control | Method-level ABAC, RLS policies, CORS policy | §2 ABAC, SECTEST-004, §10 CORS |
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

## 9. Incident Response & Communication

### 9.1 Incident Response Posture

- **IR-001**: A documented incident response runbook (finalized in Part 11 operationally, but its existence is a compliance requirement recorded here) must cover: suspected credential compromise (platform-level), suspected data exposure, AI guardrail bypass patterns, and acquirer-side outage handling.
- **IR-002**: Any confirmed data exposure or credential compromise affecting live payment credentials triggers a defined notification process to the affected operator and, where required by UAE regulatory/data-protection obligations, to relevant authorities.

### 9.2 Incident Communication Plan

- **IR-COMM-001**: Incident severity classification:

| Severity | Definition | Response Time | Communication |
|---|---|---|---|
| SEV-1 (Critical) | Payment processing down, data breach, regulatory violation | 15 minutes | Status page + email + SMS to all operators |
| SEV-2 (High) | Degraded payment processing, partial data exposure | 1 hour | Status page + email to affected operators |
| SEV-3 (Medium) | Non-critical feature degradation, minor security finding | 4 hours | Email to affected operators |
| SEV-4 (Low) | Cosmetic issues, minor bugs | 24 hours | Dashboard notification |

- **IR-COMM-002**: Communication channels:
  - **Status page**: Public status page (e.g., statuspage.io) showing system health for all services
  - **Email**: Automated email notifications for SEV-1/SEV-2 incidents
  - **SMS**: SMS alerts for SEV-1 incidents to on-call engineers
  - **Slack/Teams**: Internal real-time communication channel for incident response

- **IR-COMM-003**: Incident timeline documentation:
  - **Detection**: When was the incident detected? By what mechanism?
  - **Triage**: Who was notified? When did they acknowledge?
  - **Mitigation**: What actions were taken to mitigate impact?
  - **Resolution**: When was the incident resolved? What was the root cause?
  - **Post-mortem**: Within 48 hours of resolution, a post-mortem document is created covering timeline, root cause, impact, and remediation actions.

### 9.3 Incident Response Runbook

- **IR-004**: The incident response runbook includes specific playbooks for:
  - **Acquirer outage**: How to detect, how to failover, how to notify operators
  - **Database failure**: How to failover to replica, how to validate data consistency
  - **Redis failure**: How to handle cache misses, how to restore from backup
  - **NATS failure**: How to detect consumer lag, how to replay missed events
  - **AI model degradation**: How to detect, how to fallback to raw data, how to notify operators
  - **Certificate expiration**: How to detect impending expiration, how to rotate
  - **Secret compromise**: How to revoke, how to rotate, how to notify affected parties
  - **Data breach**: How to contain, how to assess scope, how to notify regulators

- **IR-005**: Each playbook includes: detection criteria, escalation path, mitigation steps, validation steps, and rollback procedures.

---

## 10. Threat Model (STRIDE-Based)

### 10.1 External Attacker Surface

| Threat | Target | Mitigation |
|---|---|---|
| **Spoofing** | API Gateway endpoints, webhook ingress | TLS 1.3 + JWT/API-key auth (§1), webhook signature verification (Part 7), mTLS for internal traffic (§1.3 AUTH-006), MFA for human users (AUTH-001) |
| **Tampering** | Payment intent amounts, routing policies, event streams | Event-sourcing immutability (PRIN-05), optimistic concurrency (Part 5 CONC-001), idempotency keys, outbox pattern (Part 3 §9.2) |
| **Repudiation** | Configuration changes, money-movement events | Two-tier audit framework (§5), immutable event stream, `PermissionDenied` logging (§2.3 AUTHZ-001), log integrity hash chains (LOGSEC-001) |
| **Information Disclosure** | Unauthorized data access, sensitive data in logs | ABAC enforcement (§2), RLS policies (SECTEST-004), encrypted fields (§4), log scrubbing (LOGSEC-002), data classification (ENC-009) |
| **Denial of Service** | Checkout path, AI Assistant, database | Rate limiting (Part 4 GW-003, Part 10 RL-001), account lockout (AUTH-007), circuit breakers (Part 3 §9.3), GPU isolation (Part 4 K8S-002), request size limits (REQ-001) |
| **Elevation of Privilege** | ABAC bypass, threshold bypass | Permission validation on every request (Part 4 GW-002), ABAC enforcement at aggregate command level (§2.2 ABAC-001), step-up re-auth for sensitive actions (ABAC-002), API key scope validation (AUTH-010) |

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
| Privilege escalation via API | A01 | Critical | Method-level ABAC, RLS (SECTEST-004) |
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
  - Format validation (email, UUIDv7, ISO 4217 currency code, ISO 8601 timestamp with 3-digit millisecond precision: `YYYY-MM-DDTHH:MM:SS.mmmZ`)
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

### 13.1 Key Ceremony Procedures

- **CEREMONY-001**: KEK ceremonies (initial creation and rotation) follow a formal procedure:
  1. **Pre-ceremony**: Ceremony script reviewed and approved by Security Admin and Compliance Reviewer (Maker/Checker). All participants confirmed. HSM initialized and verified.
  2. **Ceremony execution**: Minimum 3 participants required (Security Admin, Compliance Reviewer, DevOps Engineer). Each participant authenticates to the HSM independently. Key material generated within HSM boundary (never exported in plaintext).
  3. **Split knowledge**: KEK is split into 3 key shares (using Shamir's Secret Sharing or HSM-native split). Each participant holds one share. No single participant can reconstruct the complete key.
  4. **Post-ceremony**: All participants sign the ceremony log. HSM audit trail verified. Key shares stored in separate secure locations. Old KEK cryptographically destroyed.

- **CEREMONY-002**: Ceremony documentation includes:
  - Date, time, location
  - Participants (names, roles, authentication method)
  - HSM serial number and firmware version
  - Key generation parameters
  - Key share distribution record
  - Verification steps performed
  - Signatures of all participants

- **CEREMONY-003**: Key ceremony logs are retained permanently (compliance requirement). They are stored in append-only audit log (§5.3 AUD-003) and backed up to a separate, isolated storage system.

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
| BIZ-042 (ABAC, elevated-permission audit) | §2, §2.3 |
| BIZ-043 (KYB evidence, not platform decisioning) | §6 table row 3, Part 3 BC-03 cross-reference |
| OQ-006 (Part 2, secondary approver threshold) | §2.2 ABAC-001 — resolved: yes, threshold-based dual control |
| BR-031-1 (Part 2, tokenization not raw PAN) | §4.3 ENC-005 |
| Part 6 AI-P-002/AI-P-003 | §8 AISEC-001/002 |
| Threat model (STRIDE) | §10.1 through §10.3 |
| API key scoping to acquirer links | §11 AUTHZ-002, AUTHZ-003 |
| Secrets rotation automation | §12 SEC-ROT-001 through SEC-ROT-004 |

---

## 16. Gap Analysis Additions — Bank-Grade Security Hardening

### 16.1 Payment Token Lifecycle Management (PCI-DSS)

**TOK-001**: A `PaymentMethodToken` aggregate (within BC-05, managed by `orchestration-service`) manages the lifecycle of acquirer-issued payment method tokens, separated from connector credentials.

**TOK-002**: **PCI-DSS Token Classification (OQ-089 — Requires QSA Determination)**:
- Acquirer-issued tokens may be classified as cardholder data under PCI-DSS depending on the token format and whether it can be used to reconstruct PAN
- If tokens = cardholder data: CDE scope expands, TDE required on `payment_method_token` table, access controls tightened
- If tokens ≠ cardholder data: lighter scope
- **Platform assumes worst-case (tokens = cardholder data) for initial launch** — TDE enabled, RLS enforced, access logged
- Final classification requires QSA review before PCI-DSS assessment

**TOK-003**: Token Entity (SeaORM — Rust):

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_method_token")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub token_id: Uuid,
    pub payment_method_type: String,    // 'card' | 'bank_account' | 'wallet'
    pub last_four: String,
    pub card_brand: Option<String>,     // 'visa' | 'mastercard' | 'amex' | 'mada'
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub token_status: String,           // 'active' | 'expired' | 'revoked'
    pub acquirer_link_id: Uuid,
    pub acquirer_token_reference: String, // the actual token from the acquirer
    pub encrypted_token: Vec<u8>,       // envelope-encrypted
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub revocation_reason: Option<String>,
}
```

**TOK-004**: Token lifecycle commands: `StorePaymentMethodToken`, `ExpirePaymentMethodToken`, `RevokePaymentMethodToken`, `RefreshPaymentMethodToken` (for account-updater scenarios per Part 2 EX-031a).

**TOK-005**: All token access is logged to Tier 2 audit (BIZ-040). Token storage is separated from connector credentials to ensure QSA can clearly identify the cardholder data boundary.

**TOK-006**: Tokens are never returned in full via any read API. The dashboard shows last-four, brand, and expiry only.

### 16.2 Phishing-Resistant MFA Mandate

**AUTH-011**: WebAuthn (FIDO2) is the **minimum** MFA factor for Admin and Finance Operator roles. TOTP is permitted only as a secondary factor or for Developer/Read-Only roles.

**AUTH-012**: WebAuthn authenticator attestation verification: on enrollment, the platform verifies the authenticator's attestation statement to confirm it's a genuine FIDO2 device (not a software simulator).

**AUTH-013**: Session binding to WebAuthn authenticator: the session token includes a hash of the authenticator's credential ID, preventing token replay across devices.

**AUTH-014**: Account recovery flow for lost MFA devices:
- **Backup codes**: 10 single-use recovery codes generated at MFA enrollment, stored hashed server-side
- **Recovery key**: Admin-generated recovery key for emergency access
- **Identity verification**: If backup codes are lost, Admin-mediated identity verification (upload government ID + video call) with Maker/Checker approval

**AUTH-015**: Login notification: all login events (new device/IP) trigger an email notification to the principal. Suspicious login patterns (impossible travel, new country) trigger an MFA re-challenge.

**AUTH-016**: All sessions for a principal are invalidated when the principal's password is changed.

### 16.3 Database Transparent Data Encryption (TDE)

**ENC-010**: PostgreSQL databases containing Confidential or Restricted data use Transparent Data Encryption (TDE) via the `pg_tde` extension or cloud-managed TDE equivalent. TDE ensures encryption persists into WAL archives, base backups, and crash dumps — not just live volume data.

**ENC-011**: Encryption at Rest Matrix:

| Data Store | Live Volume | WAL/Logs | Backups | Replicas |
|---|---|---|---|---|
| PostgreSQL (event stores) | TDE (ENC-010) | TDE (encrypted WAL) | TDE (encrypted base backup) | TDE (encrypted replica data) |
| ClickHouse | Volume encryption | N/A (append-only) | Volume encryption at backup destination | Volume encryption |
| Redis | Volume encryption | N/A (AOF encrypted) | Volume encryption at backup destination | Volume encryption |
| OpenSearch | Volume encryption | N/A | Volume encryption | Volume encryption |
| MinIO | Per-object KMS (ENC-004) | N/A | Per-object KMS (replicated) | Per-object KMS |

### 16.4 Audit Log Tamper-Evidence

**AUD-004**: Audit log entries include cryptographic hash chaining for tamper-evidence:

```rust
pub struct AuditEntry {
    // ... existing fields from DB-004 ...
    pub previous_entry_hash: Option<Vec<u8>>,  // SHA-256 of previous entry
    pub entry_hash: Vec<u8>,                    // SHA-256 of this entry (including previous_entry_hash)
}
```

**AUD-005**: A background verification job walks the audit log chain daily and alerts on breaks (hash mismatch = tamper detected). The verification job itself is audit-logged.

**AUD-006**: Optional: periodic hash summaries anchored to an external write-once store (e.g., AWS S3 Object Lock) for independent verification.

### 16.5 Infrastructure Component Security Baseline

**INFRA-SEC-001**: All infrastructure components require explicit security configuration:

| Component | Required Controls |
|---|---|
| **Redis** | AUTH/TLS (Part 4 §10.1 REDIS-ENC-001/002/003), ACL with least-privilege roles, `protected-mode yes`, no default password |
| **OpenSearch** | TLS for all connections, basic auth or mTLS, index-level security, audit logging, network policy restricting to `ai-assistant-service` only |
| **ClickHouse** | TLS for client connections, per-service read-only users, network policy restricting to `analytics-service` and `ai-assistant-service` |
| **MinIO** | TLS, access key/secret auth, per-service IAM policies, network policy restricting to `document-service`, `compliance-service`, `reconciliation-service`, `analytics-service` |
| **NATS** | TLS 1.3 + mTLS (Part 4 §10.1 NATS-ENC-001/002/003) |
| **PostgreSQL** | TDE (ENC-010), `sslmode=verify-full` (not just `require`), per-service roles, `pg_hba.conf` restricting to authorized service IPs |

**INFRA-SEC-002**: First-boot credential rotation: all infrastructure components (MinIO, OpenSearch, Redis, ClickHouse, PostgreSQL) must have default credentials rotated on first deployment. Default credentials (e.g., MinIO `minioadmin:minioadmin`) must never exist in production.

**INFRA-SEC-003**: K8s Secrets encryption at rest: Kubernetes etcd encryption enabled via KMS provider. Preference for CSI Secret Store Driver (direct Vault mount) over Kubernetes Secrets.

### 16.6 Session Security Hardening

**SESS-SEC-001**: Session cookies must have `SameSite=Strict; Secure; HttpOnly` attributes. Cookie domain scoped to the exact deployment domain (no wildcard domains).

**SESS-SEC-002**: JWT tokens include `aud` claim binding to specific client type (dashboard vs. API). Tokens issued for the dashboard cannot be used against the API and vice versa.

**SESS-SEC-003**: Refresh token rotation with concurrent session detection: if a refresh token is used after rotation (indicating potential theft), both the old and new sessions are invalidated and the principal is notified.

**SESS-SEC-004**: Admin session revocation: Admins can revoke all sessions for any principal via `/v1/principals/{id}/sessions/revoke-all`.

### 16.7 CSRF Protection

**CSRF-001**: All state-changing API endpoints that accept cookie-based authentication require a CSRF token. The CSRF token is: (a) generated per-session, (b) included in a custom header (`X-CSRF-Token`) on every mutating request, (c) validated against the session's stored token at the API Gateway.

**CSRF-002**: For API-key-authenticated requests, CSRF protection is not required (API keys are not auto-sent by browsers).

### 16.8 Credential Access Monitoring

**CRED-MON-001**: Every decrypt/access of `merchant_acquirer_link.encrypted_config` generates an `AcquirerCredentialAccessed` audit event (Tier 2) with: actor_id, timestamp, source_ip, access_purpose.

**CRED-MON-002**: Credential access rate limiting: maximum 10 credential decryptions per principal per hour. Exceeding the limit triggers an alert and temporary access suspension.

**CRED-MON-003**: Real-time alerting on any credential access (not just "outside business hours" — for a payment platform, credential access should always be monitored).

### 16.9 IP Allowlisting for Sensitive Operations

**ABAC-009**: IP allowlisting is **mandatory** (not optional per ABAC-007) for:
- Acquirer credential management
- KEK ceremonies
- Production database access (PAM-001)
- Routing policy changes above threshold

**ABAC-010**: IP determination uses trusted proxy chain validation (not raw X-Forwarded-For). The API Gateway validates the full proxy chain and sets a trusted `X-Real-IP` header.

### 16.10 gRPC Security Hardening

**GRPC-SEC-001**: gRPC server reflection must be disabled in production. Reflection is permitted only in dev/sandbox environments.

**GRPC-SEC-002**: Per-service internal rate limiting (configurable) enforced via gRPC interceptors. The `iam-service` `ValidatePermission` endpoint (critical path for every request) has explicit internal rate limiting to prevent DoS from a compromised service.

### 16.11 AI Model Security Enhancements

**AI-BIAS-001**: The evaluation harness (Part 6 §6) is extended with a bias test set covering: merchant size segments, geographic segments, transaction amount ranges. A fairness metric (equal accuracy across segments) is computed and monitored.

**AI-BIAS-002**: Quarterly bias audit: a random sample of AI Assistant answers is reviewed by humans for fairness and consistency across merchant segments.

**AI-HALL-001**: A secondary validation layer extracts numerical claims from AI answers and cross-checks against source documents. Claims with low source-relevance scores trigger a disclaimer: "This answer may not be fully grounded in your data."

**AI-MON-001**: Hourly sampling of answer quality (automated checks on a rotating subset of the top-50 questions). Real-time latency monitoring with p99 threshold alerting. A "circuit breaker" degrades the AI Assistant to raw-data mode if quality drops below threshold within any 1-hour window.

### 16.12 Data Residency Enforcement

**RESID-001**: Infrastructure-as-code constraints enforce UAE region: deployment templates include region-locking (cloud provider region constraints), and CI/CD pipeline validates that no resource is provisioned outside the UAE region.

**RESID-002**: Network egress controls prevent data transfer outside UAE region (egress NetworkPolicy restricting outbound to UAE-region endpoints).

**RESID-003**: Backup destination validation: backups must target UAE-region storage. Backup verification job checks destination region.

**RESID-004**: Data residency attestation as a pre-GA compliance check.

### 16.13 SFTP Settlement File Security

**SFTP-SEC-001**: SFTP credentials stored via envelope encryption (same as connector credentials per SEC-001).

**SFTP-SEC-002**: SSH host key pinning for SFTP connections (prevents MITM on settlement file ingestion).

**SFTP-SEC-003**: Settlement file hash verification (SHA-256) stored in audit log alongside the file metadata.

**SFTP-SEC-004**: Network policy restricting outbound SFTP to known acquirer IP ranges only.

### 16.14 API Key Lifecycle Automation

**APIKEY-LIFE-001**: Automated key expiry enforcement: API keys are automatically deactivated after their configured maximum age (default: 90 days).

**APIKEY-LIFE-002**: Expiry notifications: 30-day, 14-day, 7-day, 1-day warnings sent via notification-service before key expiry.

**APIKEY-LIFE-003**: Emergency key revocation can be performed by Admin without Maker/Checker (APIKEY-MKCK-003), with all revocations logged.

**APIKEY-LIFE-004**: Key age monitoring: dashboard alert if any key exceeds configured maximum age.

### 16.15 AI Log Retention & Classification

**AI-LOG-001**: AI conversation logs are classified as Confidential (Part 8 ENC-009).

**AI-LOG-002**: Retention limit aligned with financial record retention (OQ-018). Default: same as Tier 2 audit logs.

**AI-LOG-003**: AI log access restricted to Compliance Reviewer + Admin roles.

**AI-LOG-004**: AI log deletion mechanism for PDPL erasure requests (where not conflicting with financial retention requirements).

### 16.16 Kubernetes Security Hardening

**K8S-SEC-001**: ResourceQuota per namespace (CPU, memory, pod count). LimitRange per container (min/max resource requests). PodDisruptionBudget for critical services (`orchestration-service`, `connector-gateway`, `iam-service`) with `minAvailable >= 2`.

**K8S-SEC-002**: mTLS certificate lifetime: workload certificates ≤ 24 hours (auto-rotated via certificate manager). Root CA offline/air-gapped. Zero-downtime rotation via SDS (Secret Discovery Service).

### 16.17 Supply Chain Enhancements

**SUPPLY-005**: License compliance scanning (`cargo deny` license checks) as a CI gate. Copyleft licenses (GPL/AGPL) require explicit approval.

**SUPPLY-006**: SBOM uploaded to a vulnerability tracking platform (e.g., Dependency Track) for continuous post-build monitoring.

**SUPPLY-007**: Ollama runtime version included in SBOM and pinned to a specific version with integrity verification.

**SUPPLY-008**: Container registry: private registry with mTLS or token authentication; separate registries/namespaces for dev/staging/production; image retention policy (retain last N images per service).

---

## 17. Gap Analysis Additions — Round 2

### 17.1 PCI-DSS Token Classification

**PCI-TOKEN-001**: Acknowledge that acquirer-issued tokens (Visa VTS, Mastercard MDES, network tokens) are classified as "cardholder data" per PCI-DSS v4.0 and require the same controls as PAN (encryption at rest, access logging, network segmentation).

**PCI-TOKEN-002**: The `PaymentMethodToken` entity (§16.1 TOK-002) must be stored in the Cardholder Data Environment (CDE) — a logically or physically segmented network zone with: dedicated encryption keys, access limited to `orchestration-service` and `connector-gateway` only, all access logged to the CDE-specific audit trail.

**PCI-TOKEN-003**: A formal scope determination by a QSA must be completed pre-GA. The SRS must NOT assume a specific SAQ type — document the architecture's intent but defer the scoping decision.

### 17.2 Hosted Payment Page Domain Separation

**PCI-DOMAIN-001**: The hosted payment page (SVC-07) must be served from a **completely separate domain** (e.g., `pay.merchant.com` or `checkout.platform.ae`) from the admin dashboard (e.g., `admin.platform.ae`). The payment page domain must have: no cookies from the admin domain, no shared `localStorage`, a separate Content Security Policy, and CORS restrictions preventing cross-domain requests.

**PCI-DOMAIN-002**: Document the domain separation in the PCI-DSS network diagram (§17.5).

### 17.3 Cardholder Data Flow Diagram

**PCI-DFD-001**: Create a formal Cardholder Data Flow Diagram as a mandatory Part 12 appendix, showing: (1) point of card data entry (client SDK → acquirer tokenization endpoint), (2) token creation flow, (3) token storage location and encryption, (4) token usage in authorization flow, (5) token lifecycle (creation, refresh, revocation, destruction), (6) all systems that handle tokens.

### 17.4 gRPC Security Hardening — Round 2

**GRPC-SEC-003**: API Gateway HMAC-signs the `actor_context` metadata on every outbound gRPC call. Downstream services verify the signature before trusting the context. This prevents a compromised service from forging actor context in gRPC headers (mTLS authenticates the service, not the actor payload).

**GRPC-SEC-004**: gRPC server reflection is compile-time disabled in staging/production builds via a `cfg` feature flag. Reflection code is compiled out entirely — not just disabled at runtime. CI grep checks production binaries for reflection symbols.

### 17.5 Card Testing / Economic Abuse Prevention

**ABUSE-001**: Operator-level velocity checks: maximum 100 authorization attempts per 15-minute window per operator (not per API key). Zero-amount authorizations (card verification) are rate-limited separately: maximum 10 per card BIN per hour.

**ABUSE-002**: Cross-IP velocity detection: if authorization attempts for the same payment method token originate from more than 3 distinct IP addresses within 5 minutes, the attempts are flagged for fraud review and the payment method token is temporarily suspended.

**ABUSE-003**: The platform logs a `CardTestingSuspected` event when velocity thresholds are exceeded, surfacing it to the fraud dashboard and AML alert queue.

### 17.6 Network Segmentation Zones

**NET-SEG-001**: Define three network zones: (1) **CDE Zone**: Contains `orchestration-service`, `connector-gateway`, and `PaymentMethodToken` database tables. Strict access: only `api-gateway` (inbound) and `connector-gateway` (outbound to acquirers). (2) **Application Zone**: All other services. Can communicate with CDE zone only via defined gRPC interfaces. (3) **Management Zone**: Admin dashboard, CI/CD, monitoring, logging. Can read from Application zone but cannot initiate payment operations.

**NET-SEG-002**: Network zone boundaries enforced via separate Kubernetes namespaces with strict NetworkPolicies, or via separate Kubernetes clusters for CDE vs. non-CDE workloads.

### 17.7 East-West Traffic Inspection

**NET-INSPECT-001**: Deploy L7 observability providing: (1) per-service-pair request volume baselines, (2) response size monitoring (detecting unusual data volumes — potential exfiltration), (3) gRPC method-level access logging.

**NET-INSPECT-002**: Alert on anomalous patterns: service A suddenly calling service B's methods it never called before, response sizes exceeding 2σ from baseline.

### 17.8 DDoS Edge Protection

**DDOS-EDGE-001**: Deploy edge-level DDoS protection via CDN/WAF layer with: volumetric DDoS mitigation, rate limiting at the edge (before reaching the API Gateway), IP reputation filtering, bot detection, and geo-blocking for non-UAE traffic (scope).

**DDOS-EDGE-002**: API Gateway rate limits (RL-003) serve as defense-in-depth after edge protection, not as the primary DDoS mitigation.

### 17.9 Encryption Key Inventory

**KEY-INVENTORY-001**: Maintain an `encryption_key_inventory` table tracking: `key_id`, `key_type` (KEK, DEK, API_signing, webhook_HMAC), `purpose`, `owner_service`, `created_at`, `last_rotated_at`, `next_rotation_due`, `status` (active, retired, compromised), `hsm_slot`.

**KEY-INVENTORY-002**: Monthly reconciliation job verifies every key has a corresponding active secret using it, and every secret has a corresponding active key. Orphaned keys are flagged for retirement.

### 17.10 Cryptographic Operation Logging

**CRYPT-LOG-001**: Every encryption/decryption operation by the KMS client is logged to Tier 2 audit with: `operation_type` (encrypt/decrypt), `key_id`, `caller_service`, `caller_actor_id`, `timestamp`, `secret_type`, `secret_id`. Sensitive values are NEVER logged.

**CRYPT-LOG-002**: KMS audit logs are stored in append-only storage and verified daily.

### 17.11 HSM Disaster Recovery

**HSM-DR-001**: Define HSM disaster recovery: (1) active-passive HSM pair with KEK material synchronized, (2) RTO for HSM failover: <15 minutes automated, <4 hours manual from backup, (3) KEK escrow in separate air-gapped storage as last-resort recovery, (4) continuous HSM health monitoring with critical alerts.

**HSM-DR-002**: Quarterly HSM failover drills (aligned with DR-004 database restore drills).

### 17.12 DEK Rotation Audit Trail

**DEK-AUDIT-001**: Every DEK rotation (automated or manual) generates a `DEKRotated` audit event with: `service_id`, `dek_id`, `old_dek_wrapped_by`, `new_dek_wrapped_by`, `rotated_by`, `timestamp`, `verification_status`.

**DEK-AUDIT-002**: DEK rotation events are logged to the immutable audit trail and retained permanently (same as KEK ceremony logs).

### 17.13 KEK Re-Encryption Job

**SEC-REENC-001**: A `KEKReEncryptionJob` runs as a durable saga that: (1) reads all DEKs from `encrypted_config` tables, (2) re-encrypts each DEK under the new KEK, (3) atomically swaps the encrypted DEK in a single transaction, (4) logs each re-encryption to the audit trail, (5) tracks progress in a `kek_reencryption_progress` table for crash recovery.

**SEC-REENC-002**: During rotation window, the system accepts both old-KEK-wrapped and new-KEK-wrapped DEKs (dual-KEK support period, default: 24 hours). KMS client tries new KEK first, falls back to old KEK. After grace period, old KEK is destroyed per KMP-003.

### 17.14 Connector Credential Rotation

**SEC-CONN-ROT-001**: Connector credential rotation follows a two-phase protocol: (1) new credentials validated and stored alongside old credentials (`previous_config` field on `merchant_acquirer_link`), (2) deployment event causes `connector-gateway` to reload credentials, (3) after configurable grace period (default: 5 minutes), old credentials removed.

**SEC-CONN-ROT-002**: If connector doesn't support dual-key overlap, rotation flagged with `requires_drain_period = true` — system waits for all in-flight transactions for that link to reach terminal state before completing rotation.

### 17.15 JWT Security Hardening

**AUTH-017**: JWT verification rejects algorithms `none`, `HS256`, `HS384`, `HS512`. Only `RS256` or `ES256` are permitted. Algorithm is validated against a whitelist on every verification.

**AUTH-018**: MFA backup code verification: 5 failed attempts per 15-minute window triggers account lockout. Failures logged to Tier 2 audit. Code exhaustion requires Admin recovery.

**AUTH-019**: API Gateway rejects requests with credentials (API keys, secrets) in URL query strings (HTTP 400). Prohibition documented in API spec and SDK docs.

### 17.16 NATS Subject Injection Prevention

**NATS-SEC-001**: All NATS subject segments validated against strict allowlist pattern `^[a-z][a-z0-9_]{0,63}$`. No user data interpolated into subject names without validation. CI lint on all `publish()` calls.

### 17.17 Protobuf Deserialization Safety

**DESER-001**: Bounded recursion depth (max 32 levels) and allocation limits for protobuf deserialization. Fuzzing targets for all protobuf types as CI gate.

### 17.18 Envelope Encryption AAD

**ENC-012**: DEKs generated via HKDF-SHA256, per-service-scoped. Additional Authenticated Data (AAD) bound to `operator_id + resource_id` to prevent ciphertext transplantation between contexts.

### 17.19 Kubernetes Admission Controller

**K8S-SEC-003**: Webhook admission controller configured with `failurePolicy: Fail` (fail-closed). PSA restricted profile as defense-in-depth layer independent of webhooks.

### 17.20 API Credentials in URLs

**APISEC-011**: OpenAPI/Swagger UI endpoints disabled in production builds via compile-time cfg. CI check: production responses don't contain OpenAPI paths.

### 17.21 Log Security — Round 2

**LOG-SEC-002**: All HTTP access log formats redact `Authorization` and `Cookie` headers. CI test: grep access log samples for JWT patterns.

**LOG-SEC-003**: Application logs signed at generation time (HMAC per entry). SIEM verifies signatures on ingestion. Or ship to write-once storage before processing.

**LOG-SEC-004**: Sanitize global panic hook — strip JWT patterns, API keys, secrets from panic output. `panic = 'abort'` for services with external restart.

### 17.22 SSRF — Round 2

**SSRF-003**: Document URL fetching: disable auto-redirect or re-validate redirect target IP. DNS resolve before request + re-validate after redirect. Max response size enforced. Content-Type validation. HTTPS-only.

**SSRF-004**: Webhook callback URLs: fresh DNS resolution at delivery time with IP validation. IPv6-mapped deny-list. Single canonical URL parsing library. Reject userinfo component, non-standard ports.

### 17.23 PCI-DSS Network Diagram

**PCI-NETDIAG-001**: Create a formal PCI-DSS network diagram as a mandatory Part 12 appendix showing: CDE boundary, Application zone, Management zone, all ingress/egress points, network segmentation controls, encryption points. Updated with every network topology change and reviewed quarterly.

### 17.24 Mass API Key Revocation

**APIKEY-LIFE-005**: `POST /v1/operator/api-keys/revoke-all` endpoint immediately revokes all active API keys in a single transaction, invalidates all active sessions, and sends emergency notification. Requires operator re-authentication confirmation.

### 17.25 Data Portability Export

**UC-080**: Data Portability Export — `POST /v1/operator/data-export` produces a structured archive (JSONL for events, CSV for settlements, original files for MinIO objects) with a 7-day download window. Satisfies potential PDPL Article 17 portability requirements. Include in M7 deliverables.

### 17.26 Break-Glass Support Access

**PAM-004**: A `support-reader` PostgreSQL role with read-only access to all operator tables (no event_store write, no outbox write) provisioned via PAM with 2-hour auto-expiry. Every query under this role logged to audit_log with `actor_type = 'support'`.

### 17.27 Secrets Rotation Runbook

**RUNBOOK-SEC-001**: Dedicated `Secrets Rotation Runbook` covering each secret type (KEK, DEK, connector API key, platform API key, database password, SFTP credentials) with: prerequisites, rotation procedure, validation checklist, rollback steps, post-rotation monitoring, required notifications. Mandatory pre-GA compliance document.

---

## 18. Open Items Carried Forward

- **OQ-018**: Confirm exact financial-record retention period (§5.2 AUD-001) with UAE legal counsel.
- **OQ-019**: Confirm whether PDPL-style data-subject erasure requests are applicable to platform-processed payment/financial records.
- **OQ-044**: Finalize KEK rotation schedule (§12 SEC-ROT-001, default 90 days) against operational risk assessment — more frequent rotation increases security but adds KMS load.
- **OQ-045**: Confirm whether API key scoping to acquirer links (§11 AUTHZ-002) is required for initial launch or deferred to H2 — depends on pilot merchant integration complexity.

---

*End of Part 8. Proceed to Part 9: Database Design.*
