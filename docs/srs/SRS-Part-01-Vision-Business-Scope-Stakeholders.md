# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 1 of 12:** Vision, Business Requirements, Scope & Stakeholders
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Document Title | Payment Orchestration & AI Assistant Platform — SRS |
| Part | 1 of 12 — Vision, Business Requirements, Scope, Stakeholders |
| Version | 0.1 (Draft) |
| Primary Region | United Arab Emirates (UAE) |
| Expansion Regions | GCC (KSA, Bahrain, Oman, Kuwait, Qatar), broader MENA, selected APAC markets (future) |
| Custody Model | **No payment custody** — software orchestration, routing, and reconciliation only. All funds flow between licensed acquirers, PSPs, and banks; the platform never becomes a holder of client/merchant funds. |
| Tenancy Model | Single-tenant deployment (one merchant/operator per deployment) |
| Architecture Style | API-first, Domain-Driven Design, Microservices, Event-Driven (NATS JetStream), CQRS |
| Core Language/Runtime | Rust (all backend services) |
| ORM Layer | SeaORM (all services) — no raw SQL in application code |
| AI Stack | Ollama-hosted Qwen3 32B (reasoning), Qwen3-VL 8B (vision/OCR), BGE-M3 (embeddings) + reranker (RAG) |
| Related Documents | Part 2 (Use Cases), Part 3 (DDD), Part 4 (Microservices), Part 5 (Orchestration Engine), Part 6 (AI Assistant/RAG), Part 7 (Gateway Connectors), Part 8 (Security/Compliance), Part 9 (Database Design), Part 10 (APIs/gRPC), Part 11 (Testing/DevOps), Part 12 (Appendices/Roadmap) |

### 0.1 Revision History

| Version | Date | Author | Description |
|---|---|---|---|
| 0.1 | Draft | Engineering/Product | Initial issue of Part 1 — vision, business requirements, scope, stakeholders |

### 0.2 How to Read This Document

This is Part 1 of a 12-part SRS series. Each part is self-contained but cross-references the others. Requirement IDs use the prefix pattern `[DOMAIN]-[NUMBER]` (e.g., `BIZ-014`, `SCOPE-007`) so that later parts (use cases, DDD, APIs) can trace back to the business requirement that motivates them. A full traceability matrix will be assembled in Part 12 (Appendices).

---

## 1. Purpose of This Document

This SRS defines the complete functional, non-functional, architectural, and operational requirements for a **single-tenant, AI-native payment orchestration platform**. The platform enables a merchant, platform, or payment service provider operating in the UAE (with expansion to the wider GCC and MENA region) to:

1. Accept, route, and reconcile payments across multiple acquirers, PSPs, and payment rails **without the platform ever holding merchant or customer funds**.
2. Manage the full payment lifecycle — invoices, payment links, subscriptions, settlements — through a unified, API-first control plane.
3. Interact with an **AI Payment Assistant** capable of answering operational questions, investigating transaction anomalies, drafting reports, and assisting with reconciliation using retrieval-augmented generation (RAG) over the operator's own data.
4. Operate under UAE regulatory expectations (Central Bank of the UAE retail payment services regulation, AML/CFT obligations, data residency expectations) while retaining an architecture capable of expanding to additional jurisdictions without a rewrite.

This Part 1 establishes **why** the system exists, **what** business outcomes it must produce, and **who** it serves. It intentionally avoids prescribing technical solutions (covered in Parts 3–11) except where the business requirement directly constrains architecture (e.g., "no custody" is a business/regulatory requirement with deep architectural consequences, so it is called out here and expanded technically later).

---

## 2. Product Vision

### 2.1 Vision Statement

> To become the operating system for payment orchestration in the UAE and the wider GCC — a platform where merchants and platforms connect once and gain intelligent, unified access to every acquirer, PSP, and payment method, with an AI assistant that understands their business and their money movement as well as a senior finance operations analyst would.

### 2.2 Problem Statement

Merchants and platforms operating in the UAE and the region today face a fragmented payment landscape:

- **BIZ-001**: Merchants typically integrate with 2–5 separate acquirers/PSPs (e.g., Network International, Telr, PayTabs, Checkout.com, local bank acquiring) to get acceptable authorization rates, redundancy, and card-scheme/local-rail coverage (mada-style domestic schemes, UAEFTS, AANI instant payments), each with different APIs, settlement cycles, and reporting formats.
- **BIZ-002**: Reconciliation across acquirers, bank settlement files, and internal ledgers is manual, spreadsheet-heavy, and error-prone, consuming significant finance operations effort every month-end.
- **BIZ-003**: There is no unified, real-time view of transaction health (authorization rates, decline reasons, fraud flags) across providers — each provider's dashboard is siloed.
- **BIZ-004**: Smaller PSPs and platforms lack the engineering capacity to build a robust orchestration and failover layer (e.g., automatic retry on a secondary acquirer when the primary declines or is down), so they either accept lower reliability or over-invest in custom infrastructure.
- **BIZ-005**: Existing dashboards require manual investigation of anomalies (e.g., "why did authorization rate drop 8% yesterday for Visa on Acquirer B?"); there is no assistant that can reason over the merchant's own transaction data and answer in natural language.
- **BIZ-006**: Regulatory and audit requirements (UAE Central Bank retail payment services regulation, VARA where relevant, AML/CFT record-keeping) demand strong audit trails, but most orchestration tooling in the market is not built with compliance-grade logging and immutability from day one.

### 2.3 Product Vision Pillars

1. **Orchestration, not custody.** The platform routes and coordinates payments across licensed, regulated providers. It never becomes a money transmitter or payment institution itself with respect to end-customer funds; all settlement of funds occurs directly between the merchant's acquirer/bank and the merchant. This is a foundational business and legal constraint, detailed in §6.
2. **API-first, headless by default.** Every capability exposed in the dashboard must first exist as a versioned REST and gRPC API. The dashboard is a reference client, not the product boundary.
3. **AI as a first-class operator, not a bolt-on chatbot.** The AI Payment Assistant is built on a RAG architecture directly over the operator's own domain events, ledger entries, and reconciliation state, using locally-hosted models (Ollama + Qwen3 family) to preserve data residency and reduce dependency on third-party model providers for sensitive financial data.
4. **UAE-first, region-ready.** Domain models separate "core payment orchestration concepts" (currency-agnostic, rail-agnostic) from "jurisdictional adapters" (UAE Central Bank rules, AANI instant payment rail, VAT invoicing rules) so that adding Saudi Arabia (SAMA, mada, SARIE) or another GCC market is a matter of adding adapters, not redesigning bounded contexts.

### 2.4 Product Vision — What This Product Is NOT

To keep scope honest, the following are explicitly **out of vision** (see also §7, Out of Scope):

- The platform is **not** a licensed payment institution, e-money issuer, or acquiring bank. It does not hold a UAE Central Bank retail payment services license itself in the base architecture; where the operator requires the platform operator to hold such a license (e.g., to operate as a payment aggregator), that is a distinct legal/business track outside this SRS's engineering scope, though the architecture must not preclude it (see §6.4).
- The platform is **not** a card scheme, is **not** a domestic switch, and does **not** replace UAEFTS/AANI — it integrates with and routes through them via licensed partners.
- The platform is **not** a general-purpose LLM chat product; the AI Assistant is scoped to payment operations, reconciliation, merchant support, and reporting tasks grounded in the tenant's own data.

---

## 3. Business Goals & Objectives

Business goals are grouped into three horizons. Each goal has an associated objective and, where meaningful at this stage, a target metric. Detailed KPI instrumentation is specified in Part 6 (Analytics) — here we record the *business* target, not the *telemetry design*.

### 3.1 Horizon 1 — MVP / Market Entry (UAE)

| ID | Goal | Objective / Target |
|---|---|---|
| GOAL-001 | Launch orchestration across top UAE acquirers | Live integrations with a minimum of 3 UAE-relevant acquirers/PSPs at GA (e.g., Network International, Telr, PayTabs, or Checkout.com MENA) |
| GOAL-002 | Reduce failed-payment revenue leakage | Provide automatic failover/retry across acquirers to recover a target of 15–25% of otherwise-lost transactions due to single-provider declines |
| GOAL-003 | Cut manual reconciliation effort | Reduce merchant finance-ops manual reconciliation time by a target of 70% via automated settlement matching |
| GOAL-004 | Establish AI Assistant baseline value | AI Assistant answers a defined set of "top 50" operational questions (reconciliation status, decline reason breakdown, settlement ETA, etc.) with accuracy validated against ground truth in QA |
| GOAL-005 | Achieve compliance-grade auditability | Every money-movement-relevant domain event is immutably recorded with actor, timestamp, and reason from day one (no retrofitting audit later) |

### 3.2 Horizon 2 — GCC Expansion

| ID | Goal | Objective / Target |
|---|---|---|
| GOAL-006 | Add Saudi Arabia support | Support mada scheme routing and SAMA-relevant reporting adapters without modifying core orchestration domain model |
| GOAL-007 | Multi-currency settlement reporting | Support multi-currency ledgers and FX-aware reconciliation across at least AED, SAR, USD |

### 3.3 Horizon 3 — Platform Maturity

| ID | Goal | Objective / Target |
|---|---|---|
| GOAL-009 | Predictive routing | Use historical authorization-rate data per acquirer/BIN/currency to dynamically select optimal routing paths (smart routing), not just static failover |
| GOAL-010 | Proactive AI operations | AI Assistant proactively surfaces anomalies (e.g., "authorization rate for Mastercard via Acquirer B dropped 12% in the last hour") rather than only answering on demand |
| GOAL-011 | Ecosystem/SDK maturity | Public SDKs (server + client) in at least 3 languages, with a partner/developer ecosystem around gateway connectors |

### 3.4 Non-Goals (explicitly not business goals of this system)

- Becoming a consumer-facing wallet or neobank.
- Issuing cards or providing card-issuing BIN sponsorship.
- Providing lending, BNPL underwriting, or credit risk decisioning as a core product (may be a future adjacent product, not in this SRS).

---

## 4. Business Requirements

Business requirements are the "why" that drives functional requirements in later parts. Each is tagged with priority (MoSCoW: Must/Should/Could/Won't for MVP) and the horizon it targets.

### 4.1 Core Business Requirements — Orchestration & Money Movement

| ID | Requirement | Priority | Horizon |
|---|---|---|---|
| BIZ-010 | The platform must allow a tenant to connect multiple acquirers/PSPs and define routing rules (priority order, cost-based, success-rate-based) without any code change — configuration only. | Must | H1 |
| BIZ-011 | The platform must never take custody of end-customer or merchant funds; all fund movement occurs directly between the acquirer/bank and the merchant's settlement account. | Must | H1 |
| BIZ-012 | The platform must support automatic failover to a secondary acquirer/PSP when the primary declines, times out, or is in a degraded state, according to tenant-configured rules. | Must | H1 |
| BIZ-013 | The platform must provide a unified transaction ledger (read model) reconciling data from all connected acquirers against the tenant's internal order/invoice records. | Must | H1 |
| BIZ-014 | The platform must support invoicing and payment-link generation as first-class products, not just raw transaction processing. | Must | H1 |
| BIZ-015 | The platform must support recurring billing / subscription orchestration (retry logic for failed renewals, dunning workflows). | Must | H1 |
| BIZ-016 | The platform must support multi-currency transactions with accurate FX recording for reconciliation (not FX conversion/settlement itself, which remains with licensed providers). | Should | H2 |

### 4.2 AI & Intelligence Business Requirements

| ID | Requirement | Priority | Horizon |
|---|---|---|---|
| BIZ-020 | The platform must provide an AI Payment Assistant capable of answering natural-language operational questions grounded in the tenant's own transaction, settlement, and reconciliation data. | Must | H1 |
| BIZ-021 | The AI Assistant must run on infrastructure the platform operator controls (self-hosted via Ollama), so that sensitive financial data is not sent to third-party model APIs by default. | Must | H1 |
| BIZ-022 | The AI Assistant must be able to process scanned/photographed documents (e.g., bank settlement advices, merchant-uploaded reconciliation files) via vision-capable models and extract structured data. | Should | H1 |
| BIZ-023 | The AI Assistant must be able to cite the specific transactions/events it used to produce an answer (auditability of AI output), not present unattributed conclusions. | Must | H1 |
| BIZ-024 | The platform must support proactive anomaly detection and alerting (e.g., authorization rate drops, unusual decline patterns) as an evolution of the AI Assistant. | Could | H3 |

### 4.3 Security & Operational Requirements

| ID | Requirement | Priority | Horizon |
|---|---|---|---|
| BIZ-040 | The platform must maintain immutable audit logs of all configuration changes, routing decisions, and money-movement-relevant events, retained per UAE regulatory retention expectations (see Part 8 for specifics). | Must | H1 |
| BIZ-041 | The platform must support data residency controls appropriate to UAE data protection expectations (PDPL) and, where applicable, sector-specific guidance for payment data. | Must | H1 |
| BIZ-042 | The platform must support attribute-based access control so that sensitive operations (e.g., changing settlement bank details) require elevated permissions and produce audit trail entries. | Must | H1 |
| BIZ-043 | The platform must support KYC/KYB evidence storage and status tracking for merchants (evidence storage and workflow only; the platform does not perform its own regulated KYC/KYB decisioning — this is delegated to a licensed partner or the tenant's own compliance process, unless/until the platform itself is licensed). | Must | H1 |
| BIZ-044 | The platform must comply with OWASP Top 10 (2021) security controls and be assessed against PCI-DSS 4.0 requirements appropriate to its scope (SAQ-A or SAQ-A-EP). | Must | H1 |
| BIZ-045 | The platform must implement AML/CFT transaction monitoring with rule-based suspicious activity detection and SAR generation capability. | Must | H1 |
| BIZ-046 | The platform must implement fraud scoring integration with real-time risk assessment before authorization attempts. | Must | H1 |
| BIZ-047 | The platform must generate regulatory reports required by UAE Central Bank (transaction volumes, chargeback rates, system availability). | Must | H1 |
| BIZ-048 | The platform must implement defense-in-depth security: database-level row-level security, container hardening, network policies, and security headers. | Must | H1 |
| BIZ-049 | The platform must implement supply chain security: SBOM generation, dependency pinning, container image signing, and CVE response SLAs. | Must | H1 |
| BIZ-050 | The platform must implement privileged access management: just-in-time access, session recording, and no standing production access. | Should | H1 |

### 4.4 Compliance & Trust Requirements

| ID | Requirement | Priority | Horizon |
|---|---|---|---|
| BIZ-051 | The platform must support data retention automation: scheduled jobs to archive/purge data exceeding configured retention periods, with audit logging of all retention actions. | Must | H1 |
| BIZ-052 | The platform must support graceful shutdown for all services: stop accepting new requests, complete in-flight requests, flush event store writes, and deregister from service discovery before termination. | Must | H1 |
| BIZ-053 | The platform must support health check endpoints for all services: liveness, readiness, startup, and deep health checks for Kubernetes orchestration. | Must | H1 |
| BIZ-054 | The platform must support leader election for scheduled jobs to prevent duplicate execution when multiple service replicas are running. | Must | H1 |
| BIZ-055 | The platform must support event replay capability: ability to replay events for a specific aggregate, rebuild read models from scratch, and handle event store corruption recovery. | Must | H1 |
| BIZ-056 | The platform must support webhook delivery status tracking: record delivery attempts, successes, failures, and provide a dashboard for manual replay of failed deliveries. | Must | H1 |
| BIZ-057 | The platform must support model version pinning: hash verification of AI model weights, version tracking, and rollback procedure for bad model updates. | Must | H1 |

### 4.5 Business Requirements Traceability Note

Each `BIZ-xxx` ID above will be referenced from: Part 2 (which use cases satisfy it), Part 3 (which bounded context/aggregate enforces it), and Part 12 (final traceability matrix). Do not renumber these IDs in later parts — append new IDs rather than reusing numbers.

---

## 5. Market Context Summary (UAE)

*(A full market analysis with sizing, competitive landscape, and regulatory citations is intentionally kept brief here and will be expanded as a dedicated appendix in Part 12; this section provides only the context necessary to justify the business requirements above.)*

- **MKT-001**: The UAE payments market is characterized by strong card-scheme penetration (Visa/Mastercard dominant), growing domestic instant-payment rail adoption (AANI, launched by the Central Bank of the UAE), and a competitive acquiring landscape (Network International, Magnati, Telr, PayTabs, Checkout.com, Stripe's regional partnerships, among others).
- **MKT-002**: UAE Central Bank regulation of retail payment services (the Retail Payment Services and Card Schemes Regulation) creates defined categories of licensed activity (e.g., payment account issuance, payment aggregation, merchant acquiring). A pure orchestration/software layer that does not touch funds is designed to sit outside the funds-custody-triggering categories, but this must be validated with UAE legal counsel per tenant business model (see §6.4 — this SRS records the *architectural* requirement to support that legal position, not the legal opinion itself).
- **MKT-003**: Demand signal for orchestration/failover comes from the fact that individual acquirers in the region can have variable authorization rates and occasional platform-level outages; merchants with single-acquirer dependency report material revenue impact during such events.
- **MKT-004**: AI adoption in financial operations is nascent in the region; a compliant, data-resident AI assistant is a differentiator versus assistants built on third-party cloud LLM APIs, given regional sensitivity around financial data leaving the country/region.

---

## 6. The "No Custody" Constraint — Business Rationale and Architectural Implication

Because this single business requirement (BIZ-011) shapes almost every later architectural decision, it is elevated to its own section rather than left as a single table row.

### 6.1 Definition

"No payment custody" means: **at no point does the platform's own bank account, e-money balance, or ledger become the legal holder of funds belonging to a merchant or an end customer.** The platform:

- Initiates and orchestrates payment instructions (authorize, capture, refund, void, settle-triggering) via APIs to licensed acquirers/PSPs/banks.
- Records what happened (a ledger of *record of orchestration and reconciliation*, not a ledger of *fund ownership*).
- Never nets, pools, or commingles merchant funds in an account it controls.

### 6.2 Business Rationale

- **CUST-001**: Avoids the platform itself needing a payment institution / money transmitter license in every jurisdiction it operates, which would dramatically slow expansion (GOAL-006 onward) and increase regulatory capital requirements.
- **CUST-002**: Reduces the platform's own AML/CFT and safeguarding obligations to those of a technology/data processor rather than a funds-holding institution — though KYC/KYB *evidence workflow support* (BIZ-043) and audit logging (BIZ-040) are still required because the platform is a control point in the payment flow even without custody.
- **CUST-003**: Aligns with how UAE Central Bank's Retail Payment Services regulation defines licensable categories; a pure "payment initiation/orchestration + reporting" role is more likely to sit outside the categories that require a license, subject to legal confirmation per §6.4.

### 6.3 Architectural Implication (forward reference)

Because of BIZ-011/CUST-001–003:

- The **Payment Orchestration Engine** (Part 5) is modeled as a *state machine and router*, never as a *ledger of owned funds*. Its "ledger" is a reconciliation/read-model ledger, not a general ledger of the platform's own liabilities to merchants.
- Settlement Service (Part 4/5) reconciles *bank/acquirer settlement files* against *internal order state*; it does not compute or hold "amount owed to merchant by platform," because the platform does not owe merchants funds — the acquirer/bank does.

### 6.4 Explicit Assumption Requiring Legal Sign-off

**ASSUMP-001**: This SRS assumes UAE legal counsel will confirm whether the "no custody, orchestration only" posture keeps the *platform operator* outside UAE Central Bank licensable categories. Where the platform operator's business model requires a license, that is a business/legal decision outside engineering scope, and the architecture must be extensible to a "licensed mode" without a full rewrite.

---

## 7. Scope

### 7.1 In Scope (MVP / Horizon 1 unless marked H2/H3)

- **SCOPE-001**: Merchant onboarding, KYC/KYB evidence workflow (not decisioning), operator configuration.
- **SCOPE-002**: Connector framework for acquirer/PSP integration (initially 3+ UAE-relevant providers) — see Part 7.
- **SCOPE-003**: Payment orchestration: authorize, capture, void, refund, retry/failover routing — see Part 5.
- **SCOPE-004**: Invoice generation and payment-link generation, including branded/hosted payment pages.
- **SCOPE-005**: Subscription/recurring billing orchestration with dunning.
- **SCOPE-006**: Reconciliation engine matching acquirer settlement files/webhooks to internal order/invoice records.
- **SCOPE-007**: Unified transaction/analytics dashboard (backed by ClickHouse) — authorization rates, decline reasons, settlement status.
- **SCOPE-008**: AI Payment Assistant (RAG over tenant's own operational data; Ollama-hosted Qwen3 32B + Qwen3-VL 8B for document/vision tasks; BGE-M3 + reranker for retrieval) — see Part 6.
- **SCOPE-009**: Document management for compliance evidence, settlement advices, and merchant-uploaded files (MinIO-backed object storage) with OCR pipeline.
- **SCOPE-010**: Identity & Access Management: authentication, ABAC, audit logging — see Part 8.
- **SCOPE-011**: Webhooks and SDKs for merchant/platform integration — see Part 10.
- **SCOPE-012**: Fraud/risk scoring signals surfaced to merchants (initially rule-based/heuristic; ML-based risk scoring is H2/H3) — see Part 5/6.
- **SCOPE-013 (H2)**: Multi-currency reconciliation, GCC expansion adapters (Saudi Arabia first).

### 7.2 Out of Scope (explicitly, for this SRS and the foreseeable roadmap)

- **OOS-001**: The platform operator acting as a licensed acquiring bank, card issuer, or e-money institution holding customer/merchant funds in its own name, under the base architecture described in this SRS (see §6.4 for the conditional extensibility note).
- **OOS-002**: Card issuing / BIN sponsorship.
- **OOS-003**: Consumer-facing wallet, neobank, or savings/lending products.
- **OOS-004**: Credit underwriting or BNPL risk decisioning as a core product.
- **OOS-005**: Building a competing card scheme or domestic switch — the platform integrates with existing rails (card schemes, AANI, UAEFTS) rather than replacing them.
- **OOS-006**: General-purpose consumer chatbot functionality unrelated to payment operations.
- **OOS-007**: On-premises deployment into merchant-owned data centers (the platform is delivered as SaaS; a private-cloud/dedicated-tenant deployment model may be considered later but is not designed for in this SRS's MVP scope).

### 7.3 Scope Boundary Notes

- Where a capability is "Should" or "Could" priority for H1 (see §4 tables) but full design is deferred, Parts 3–11 will still model the domain so that adding the capability later does not require breaking changes to core aggregates (this is an explicit non-functional requirement — see Part 8, "Extensibility").
- AI Assistant scope (SCOPE-008) is grounded strictly in **the tenant's own operational data plus platform documentation**; it is not a general financial advisory product and must not be positioned as providing regulated financial advice (see Part 6, AI Assistant guardrails, and Part 8, compliance framing).

---

## 8. Stakeholders

### 8.1 Stakeholder Register

| ID | Stakeholder | Category | Primary Interest |
|---|---|---|---|
| STK-001 | Merchant Finance/Ops Team | External — Operator User | Fast, accurate reconciliation; clear settlement visibility; fewer manual spreadsheets |
| STK-002 | Merchant Engineering Team | External — Operator User | Clean API-first integration, reliable webhooks, good SDKs |
| STK-003 | Merchant Business Owner / Founder | External — Operator Decision-Maker | Revenue recovery via failover, cost transparency, trust/compliance assurance |
| STK-005 | Acquirer/PSP Partner | External — Integration Partner | Stable, well-documented connector integration; clear settlement file formats |
| STK-006 | UAE Regulator (Central Bank of the UAE) | External — Regulatory | Compliance with retail payment services regulation, AML/CFT, data residency |
| STK-007 | Platform Product Management | Internal | Feature prioritization, roadmap alignment with business goals (§3) |
| STK-008 | Platform Engineering (Backend/Rust) | Internal | Buildable, testable (TDD), maintainable DDD-aligned architecture |
| STK-009 | Platform AI/ML Team | Internal | RAG quality, model hosting (Ollama), data grounding, evaluation harness |
| STK-010 | Platform Security & Compliance Team | Internal | Audit trail completeness, access control, incident response readiness |
| STK-011 | Platform DevOps/SRE Team | Internal | Deployability, observability, SLAs, disaster recovery |
| STK-012 | QA/Test Engineering | Internal | Testability of every domain rule, acceptance criteria clarity |
| STK-013 | End Customer (Merchant's Customer) | External — Indirect | Frictionless checkout, no data over-collection, trust that payment succeeds reliably |
| STK-014 | Legal Counsel (UAE) | External Advisor | Confirming custody/licensing posture (§6.4), data protection compliance |

### 8.2 Stakeholder Needs vs. Business Requirements (Cross-Reference Preview)

| Stakeholder | Key Needs | Related BIZ IDs |
|---|---|---|
| STK-001 (Finance/Ops) | Reconciliation automation, unified ledger view | BIZ-013, BIZ-016 |
| STK-002 (Merchant Eng) | API-first, webhooks, SDKs | SCOPE-011 (detailed in Part 10) |
| STK-003 (Business Owner) | Failover/revenue recovery, transparent fees | BIZ-012 |
| STK-006 (Regulator) | Audit trail, data residency, KYC/KYB evidence | BIZ-040, BIZ-041, BIZ-043 |
| STK-009 (AI/ML Team) | Grounded, citable AI answers on self-hosted infra | BIZ-020, BIZ-021, BIZ-023 |
| STK-010 (Security/Compliance) | ABAC, immutable logs | BIZ-042, BIZ-040 |
| STK-014 (Legal) | Custody/licensing clarity | ASSUMP-001 (§6.4) |

### 8.3 RACI Summary for This Document Series

| Activity | Responsible | Accountable | Consulted | Informed |
|---|---|---|---|---|
| Business requirements (this Part) | Product Management (STK-007) | VP Product/CEO | Legal (STK-014), Merchant advisory panel | All internal stakeholders |
| DDD/domain modeling (Part 3) | Backend Engineering (STK-008) | Chief Architect | Product Management | QA, DevOps |
| AI Assistant design (Part 6) | AI/ML Team (STK-009) | Chief Architect | Security/Compliance | Product Management |
| Security/compliance requirements (Part 8) | Security & Compliance (STK-010) | CISO | Legal, Regulator liaison | All |
| Testing strategy (Part 11) | QA Engineering (STK-012) | Engineering Lead | Backend Engineering | Product Management |

---

## 9. User Personas (Preview — Full Persona Detail in Part 2)

Brief personas are introduced here because they justify business requirements; full journey maps and use cases are in Part 2.

- **Persona: "Fatima, Finance Operations Lead"** at a mid-size UAE e-commerce merchant. Needs daily reconciliation confidence and monthly close automation. Primary consumer of BIZ-013, BIZ-016, and the AI Assistant's reconciliation Q&A.
- **Persona: "Rashid, Head of Engineering"** at a merchant integrating the platform. Needs stable APIs, sandbox environment, clear webhook semantics, idempotency guarantees. Primary consumer of SCOPE-011 and Part 10.
- **Persona: "Omar, Compliance Officer"** at the platform operator (internal stakeholder acting on behalf of STK-010/STK-014). Needs audit completeness and defensible "no custody" posture documentation. Primary consumer of BIZ-040–043 and §6.

---

## 10. Assumptions, Dependencies, and Constraints

### 10.1 Assumptions

- **ASSUMP-001**: See §6.4 (custody/licensing legal confirmation, per tenant business model).
- **ASSUMP-002**: The operator will hold their own merchant acquiring relationships; the platform does not need to become a party to card scheme rules directly.
- **ASSUMP-003**: Self-hosted model inference (Ollama + Qwen3 32B / Qwen3-VL 8B) is assumed to provide acceptable latency and quality for the AI Assistant's operational use cases at MVP scale; a fallback/upgrade path (larger models, additional GPU capacity) is assumed to be available if quality benchmarks (Part 6) are not met.
- **ASSUMP-004**: UAE data residency expectations can be satisfied by hosting the full stack (Postgres, Redis, ClickHouse, OpenSearch, MinIO, Ollama) within UAE-region cloud/data-center infrastructure; specific provider selection is a Part 9/Part 11 concern.

### 10.2 Dependencies

- **DEP-001**: Availability and API stability of at least 3 UAE acquirer/PSP partners for MVP connector development (Part 7).
- **DEP-002**: Legal/compliance sign-off per §6.4 before GA launch.
- **DEP-003**: GPU/inference infrastructure capacity for Ollama-hosted models (Qwen3 32B, Qwen3-VL 8B) sized per Part 11 performance/scalability targets.
- **DEP-004**: Bank/acquirer settlement file formats and delivery mechanisms (SFTP, API, webhook) must be documented per partner for the Reconciliation Engine (Part 5/9).

### 10.3 Constraints

- **CONS-001**: All backend services must be implemented in Rust (organizational technology constraint) for performance, memory safety, and consistency. No plain SQL in application code — all database access through SeaORM.
- **CONS-002**: All domain logic must be developed test-first (TDD) — see Part 11 for standards; this is a process constraint that affects estimation and delivery cadence, recorded here because it is a business decision (quality/maintainability trade-off), not merely a technical preference.
- **CONS-003**: The platform must not, in its base architecture, require a payment institution license for the platform operator (see §6, §6.4) — all fund movement must route through licensed partners rather than internal pooled accounts.
- **CONS-004**: Primary data residency is UAE; architecture must not assume a single global region deployment (Part 9/11 will define region-aware deployment topology).

---

## 11. Success Criteria for the Overall Program

High-level program success criteria (detailed, measurable acceptance criteria per feature are in Part 11):

- **SUCC-001**: At GA, at least 3 acquirer/PSP integrations are live and passing end-to-end reconciliation tests in production for at least one pilot merchant.
- **SUCC-002**: Automated failover recovers a measurable percentage (target range in GOAL-002) of transactions that would otherwise fail on a single-provider setup, validated with real pilot merchant traffic.
- **SUCC-003**: The AI Assistant answers the "top 50" defined operational questions (final list in Part 6) with accuracy validated by a human QA review process before GA, and every answer is traceable to source events/records.
- **SUCC-004**: Zero data leakage findings in a pre-GA security review (Part 8).
- **SUCC-005**: 100% of money-movement-relevant domain events are captured in the immutable audit log with no gaps identified in a pre-GA audit trail completeness review.

---

## 12. Relationship of Part 1 to Subsequent Parts

| This Part Establishes | Expanded In |
|---|---|
| Business requirements (BIZ-xxx) | Part 2 (use cases satisfying each BIZ-xxx), Part 12 (traceability matrix) |
| "No custody" constraint (§6) | Part 5 (Payment Orchestration Engine domain model), Part 8 (compliance framing), Part 9 (ledger schema design) |
| Scope (§7) | Part 2 (detailed use case boundaries), Part 4 (microservice boundaries aligned to in-scope capabilities) |
| Stakeholders & personas (§8–9) | Part 2 (full journey maps and use case actors) |
| AI Assistant business requirements (§4.2) | Part 6 (RAG architecture, model hosting, evaluation) |
| Assumptions/Dependencies/Constraints (§10) | Part 9 (deployment/data residency), Part 11 (performance/testing implications) |

---

## 13. Open Questions for Business/Legal Sign-off Before Part 5 Finalization

These must be resolved (or explicitly deferred with owner and date) before the Payment Orchestration Engine domain model (Part 5) is finalized, since they affect aggregate boundaries:

1. **OQ-001**: Confirm with UAE legal counsel whether the base "orchestration only, no custody" model requires any platform-operator license for the MVP. Owner: STK-014. 
2. **OQ-003**: Confirm final list of MVP acquirer/PSP partners (DEP-001) so Part 7 (Gateway Connector Framework) can be scoped against real API documentation rather than generic assumptions.
3. **OQ-004**: Confirm target GPU/inference infrastructure budget and availability (DEP-003), since this materially affects which Qwen3 model variants/quantizations are feasible at target latency (to be finalized in Part 6 and Part 11).

---

*End of Part 1. Proceed to Part 2: Business Processes & Use Cases.*
