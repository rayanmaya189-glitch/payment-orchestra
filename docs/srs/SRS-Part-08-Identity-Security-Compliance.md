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
- **AUTH-002**: Session tokens are short-lived JWTs (access token) plus a longer-lived refresh token, both tenant- and principal-scoped; refresh tokens are revocable (stored server-side reference, not purely stateless) so a compromised session can be invalidated immediately — a pure stateless-refresh design was rejected specifically because it cannot be revoked before natural expiry, which is unacceptable for a financial-operations product.
- **AUTH-003**: Password policy and MFA enrollment enforcement details (minimum length, breach-list checking) are configurable per tenant's security policy tier (ties to BIZ-032 tiered commercial model — higher tiers may mandate stricter policy).

### 1.2 Machine/API Clients (Merchant Server-to-Server)

- **AUTH-004**: API key + secret pairs, scoped to specific permission sets (never a single all-powerful key), rotatable without downtime (dual-active-key overlap period supported).
- **AUTH-005**: All API keys are hashed at rest (never stored recoverable-plaintext, even encrypted) — verification is via hash comparison, following the same discipline as password storage.

### 1.3 Service-to-Service (Internal Mesh)

- **AUTH-006**: mTLS via the service mesh (Part 4 §8) provides service identity; `iam-service`'s `ValidatePermission` call additionally carries the propagated tenant/principal context (Part 4 §7 MT-001) so that internal service identity (mesh certificate) and business-actor identity (the human/API-key principal on whose behalf the call is made) are both verifiable and distinct — a compromised service credential alone cannot impersonate an arbitrary tenant's principal without also forging the propagated context, which the mesh's mTLS design prevents from being injected except by the authenticated gateway.

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

- **SEC-001**: Acquirer credentials, KYB-partner API keys, and any other third-party secrets are stored using envelope encryption: a per-tenant Data Encryption Key (DEK) encrypts the secret; the DEK itself is encrypted by a platform-wide Key Encryption Key (KEK) held in a dedicated secrets/KMS component, never co-located with application data (Part 9 elaborates the exact storage split).
- **SEC-002**: Application service credentials (database passwords, internal service tokens) are managed via the deployment platform's native secrets mechanism (Kubernetes Secrets backed by an external secrets manager, per Part 11) — never committed to source control or baked into container images, enforced by CI/CD scanning (Part 11).
- **SEC-003**: Key rotation is supported for both the KEK (platform-wide, scheduled rotation) and per-tenant DEKs (rotatable independently, e.g., on suspected compromise of a single tenant's secrets, without requiring a platform-wide re-encryption event).

---

## 4. Encryption

### 4.1 In Transit

- **ENC-001**: TLS 1.2+ for all external traffic (terminated at API Gateway, Part 4 §2); mTLS for all internal service-to-service traffic (Part 4 §8).
- **ENC-002**: Webhook endpoints (inbound from acquirers) require TLS and, per-connector, signature verification (Part 7 §1.2 `verify_webhook_signature`) as a second layer beyond transport security.

### 4.2 At Rest

- **ENC-003**: Database-level encryption at rest (Postgres, ClickHouse, OpenSearch, Redis persistence where enabled) via the underlying storage/volume encryption, plus field-level envelope encryption (§3) for the specific categories of especially sensitive data: acquirer credentials, KYB evidence file references, and any tokenized-but-still-sensitive payment method references.
- **ENC-004**: MinIO object storage (documents, exported reports) is encrypted at rest with per-object or per-bucket keys managed through the same KMS component as §3.

### 4.3 PCI-DSS Scope Minimization

- **ENC-005**: The platform never stores raw PAN (card numbers) — checkout flows tokenize directly with the acquirer/PSP or via a PCI-compliant tokenization provider, and only acquirer-issued tokens are stored (BR-031-1, Part 2). This is a deliberate architectural choice to keep the platform's own PCI-DSS scope as SAQ-A-adjacent (transmission/redirect only) rather than a full cardholder-data-environment scope — final PCI scoping determination requires a qualified assessor's review, which is a compliance-process activity outside this SRS's engineering scope, but the architecture is designed specifically to make that lighter scope achievable.

---

## 5. Audit Framework

### 5.1 Two-Tier Audit Model (Restated and Detailed from Part 3 PRIN-05)

| Tier | Applies To | Mechanism |
|---|---|---|
| Tier 1 — Event-sourced audit (source of truth) | BC-05, BC-08, BC-09, BC-10, BC-16 | The domain event stream itself (Part 3 §4 envelope: actor, timestamp, causation/correlation IDs) |
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
| UAE Central Bank Retail Payment Services Regulation — licensable-activity boundary | Part 1 §6 no-custody architecture; Part 3 structural absence of platform-owned-balance aggregates; Part 5/16 marketplace-split routed through licensed partner only |
| AML/CFT record-keeping | §5 audit framework (Tier 1/2), §5.2 retention floor |
| KYC/KYB evidence handling | Part 2 UC-002, Part 3 BC-03, this Part §3/§4 (evidence encryption) |
| Data residency (UAE PDPL and sector expectations) | Part 1 ASSUMP-004; full stack (Postgres/Redis/ClickHouse/OpenSearch/MinIO/Ollama) deployed within UAE-region infrastructure (Part 9/11 finalize provider/region specifics) |
| Consumer data protection (PDPL) | RBAC/ABAC scoping (§2), encryption (§4), and — where individual end-customer data subject rights apply — a data-subject-request handling process that must reconcile with AUD-001's financial-record retention floor (flagged as a legal/process decision, not purely technical) |
| Card scheme compliance (PCI-DSS) | §4.3 scope minimization |

---

## 7. Multi-Tenant Isolation — Security Testing Requirements (Preview, Full Detail in Part 11)

- **SECTEST-001**: A dedicated cross-tenant isolation test suite (referenced already in Part 6 §6.2 for the AI Assistant specifically) must run against every release: automated attempts to access another tenant's data via every API surface, using deliberately similar/adjacent test fixtures across tenants to catch subtle leakage (e.g., off-by-one tenant-ID bugs, cache-key collisions).
- **SECTEST-002**: Periodic third-party penetration testing (cadence to be set in Part 11) covering the API Gateway, AI Gateway, and connector webhook endpoints specifically, given their exposure to untrusted/external input.
- **SECTEST-003**: Static analysis and dependency vulnerability scanning integrated into CI/CD (Part 11) — Rust's memory-safety guarantees reduce but do not eliminate the need for this (logic-level vulnerabilities, e.g., authorization bypass, are not caught by memory safety).

---

## 8. AI-Specific Security Controls (Cross-Reference to Part 6)

This Part is the authoritative home for the *security/compliance framing* of controls whose *mechanism* is detailed in Part 6:

- **AISEC-001**: Tenant isolation of retrieval indices (Part 6 §1 AI-P-002) is a security control, validated by SECTEST-001's cross-tenant suite extended to include AI Assistant query paths specifically.
- **AISEC-002**: The AI guardrail audit log (Part 4 §3.2 AIGW-004, Part 6 §3.2 step 6) is a Tier 2 audit mechanism (§5.1 above) and subject to the same retention floor (§5.2).
- **AISEC-003**: Prompt-injection screening (Part 6 §5.1 GRD-IN-001) is treated as a security control subject to the same penetration-testing cadence as other externally-exposed input surfaces (§7 SECTEST-002).

---

## 9. Incident Response Posture (Preview)

- **IR-001**: A documented incident response runbook (finalized in Part 11 operationally, but its existence is a compliance requirement recorded here) must cover: suspected credential compromise (tenant or platform-level), suspected cross-tenant data exposure, AI guardrail bypass patterns, and acquirer-side outage handling.
- **IR-002**: Any confirmed cross-tenant data exposure or credential compromise affecting live payment credentials triggers a defined notification process to affected tenants and, where required by UAE regulatory/data-protection obligations, to relevant authorities — exact notification timelines and thresholds require legal counsel input (ties to Part 1 §6.4 pattern of flagging legal-dependent items rather than guessing at specifics).

---

## 10. Threat Model (STRIDE-Based)

### 10.1 External Attacker Surface

| Threat | Target | Mitigation |
|---|---|---|
| **Spoofing** | API Gateway endpoints, webhook ingress | TLS + JWT/API-key auth (§1), webhook signature verification (Part 7), mTLS for internal traffic (§1.3 AUTH-006) |
| **Tampering** | Payment intent amounts, routing policies | Event-sourcing immutability (PRIN-05), optimistic concurrency (Part 5 CONC-001), idempotency keys |
| **Repudiation** | Configuration changes, money-movement events | Two-tier audit framework (§5), immutable event stream, `PermissionDenied` logging (§2.3 AUTHZ-001) |
| **Information Disclosure** | Cross-tenant data leakage | Tenant-scoped queries (Part 4 MT-002), structural isolation (OS-001, MINIO-001), encrypted fields (§4), SECTEST-001 cross-tenant test suite |
| **Denial of Service** | Checkout path, AI Assistant | Rate limiting (Part 4 GW-003, Part 10 RL-001), circuit breakers (Part 3 §9.3), GPU isolation (Part 4 K8S-002) |
| **Elevation of Privilege** | RBAC bypass, ABAC threshold bypass | Permission validation on every request (Part 4 GW-002), ABAC enforcement at aggregate command level (§2.2 ABAC-001), step-up re-auth for sensitive actions (ABAC-002) |

### 10.2 Insider Threat

| Threat | Target | Mitigation |
|---|---|---|
| **Compromised admin account** | Tenant data/config modification | MFA enforcement (§1.1 AUTH-001), step-up re-auth for acquirer credential changes (ABAC-002), audit trail of all changes (§5) |
| **Malicious platform operator** | Cross-tenant data access | mTLS service identity + propagated tenant context (§1.3 AUTH-006), no super-admin bypass of tenant scoping, audit log of all admin actions |
| **Compromised AI model/inference** | Data exfiltration via model output | AI Gateway guardrails (Part 6 §5), output monitoring (GRD-IN-003), no write-path tool access (AI-P-003) |

### 10.3 Supply Chain

| Threat | Target | Mitigation |
|---|---|---|
| **Compromised acquirer connector** | Malicious response injection | ACL pattern (Part 3 §8), normalized response types (Part 7 CONN-001/002), response schema validation before aggregate state mutation |
| **Compromised Ollama model** | Prompt injection, data exfiltration | Self-hosted inference (AI-P-004), input sandboxing (GRD-IN-003), output monitoring, model integrity verification (hash checking on model load) |
| **Compromised dependency** | Remote code execution, data theft | Dependency vulnerability scanning in CI/CD (§7 SECTEST-003), Rust's memory safety reducing attack surface, `cargo audit` in pipeline |
| **Compromised NATS/stream** | Event injection, event loss | Outbox pattern (Part 3 §9.2) for publish reliability, durable consumers (Part 4 §4.2), event signature verification (future enhancement) |

---

## 11. API Key Scoping to Acquirer Links

- **AUTHZ-002**: In addition to role-based and attribute-based permission scoping (§2), API keys can optionally be scoped to specific `MerchantAcquirerLink` IDs. A scoped API key can only initiate transactions against the specified acquirer(s), following the principle of least privilege.
- **AUTHZ-003**: Unscoped API keys (no acquirer restriction) are permitted for Admin-role keys only, since Admins need full acquirer access for configuration. Developer and Finance Operator keys should be scoped to specific acquirer links where their workflow permits.

---

## 12. Secrets Rotation Automation

- **SEC-ROT-001**: Platform-level KEK (§3 SEC-001) is rotated on a configurable schedule (default: 90 days) via the KMS component. Rotation is automatic and zero-downtime — the KMS supports both old and new KEK versions during a transition window, re-encrypting DEKs transparently.
- **SEC-ROT-002**: Per-tenant DEKs are re-encrypted under the new KEK during rotation — no application-level re-encryption of data is needed because the DEK wrapping is transparent to the data layer.
- **SEC-ROT-003**: When a merchant rotates acquirer credentials, the `connector-gateway` invalidates its cached credential within one cache TTL cycle (max 30 seconds) and reloads from the encrypted store on next use.
- **SEC-ROT-004**: API key rotation (AUTH-004) supports a dual-active-key overlap period — the merchant generates a new key while the old key remains valid for a configurable grace period (default: 24 hours), allowing zero-downtime rotation.

---

## 13. Traceability

| Requirement | Realized By |
|---|---|
| BIZ-030 (tenant isolation) | §2.2 ABAC-003, §7 SECTEST-001 |
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

## 14. Open Items Carried Forward

- **OQ-018**: Confirm exact financial-record retention period (§5.2 AUD-001) with UAE legal counsel.
- **OQ-019**: Confirm whether PDPL-style data-subject erasure requests are applicable to platform-processed payment/financial records.
- **OQ-044**: Finalize KEK rotation schedule (§12 SEC-ROT-001, default 90 days) against operational risk assessment — more frequent rotation increases security but adds KMS load.
- **OQ-045**: Confirm whether API key scoping to acquirer links (§11 AUTHZ-002) is required for MVP or deferred to H2 — depends on pilot merchant integration complexity.

---

*End of Part 8. Proceed to Part 9: Database Design.*
