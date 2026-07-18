# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 2 of 12:** Business Processes & Use Cases
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 2 of 12 — Business Processes & Use Cases |
| Depends On | Part 1 (Vision, Business Requirements, Scope, Stakeholders) |
| Feeds Into | Part 3 (DDD & Bounded Contexts), Part 5 (Payment Orchestration Engine), Part 6 (AI Assistant), Part 11 (Testing/Acceptance Criteria) |
| ID Scheme | `UC-###` (use case), `PROC-###` (business process/journey), `AF-###` (alternate flow), `EX-###` (exception flow) |
| Traceability | Every use case references the `BIZ-###` / `SCOPE-###` IDs from Part 1 it satisfies |

### 0.1 Reading Guide

Section 1 gives an end-to-end process map (the "shape" of how an operator moves through the platform). Section 2 defines actors precisely (mapped to Part 1 stakeholders/personas). Sections 3–11 provide fully worked use cases, one process area at a time, each with: goal, actors, preconditions, main flow, alternate flows, exception flows, postconditions, business rules, and non-functional notes forward-referenced to later parts. Section 12 gives the full use-case-to-requirement traceability table.

---

## 1. End-to-End Process Map

At the highest level, the operator's lifecycle on the platform moves through five macro-processes:

```
PROC-01 Tenant & Merchant Onboarding
        │
        ▼
PROC-02 Acquirer/PSP Connection & Routing Configuration
        │
        ▼
PROC-03 Transaction Lifecycle (Checkout → Authorize → Capture/Void → Refund)
        │
        ├──► PROC-04 Invoicing, Payment Links & Subscription Billing (uses PROC-03 under the hood)
        │
        ▼
PROC-05 Settlement & Reconciliation
        │
        ▼
PROC-06 AI Assistant Interaction (cross-cutting — usable at any point from PROC-02 onward)
        │
        ▼
PROC-07 Dispute, Refund & Chargeback Handling
        │
        ▼
PROC-08 Reporting & Analytics
```

Two additional cross-cutting processes apply throughout: **PROC-09 Identity, Access & Audit** (every actor action in every process above is authenticated, authorized, and audit-logged).

---

## 2. Actors

Actors are precise, callable roles used consistently across all use cases in this document. Each maps back to a Part 1 stakeholder (`STK-###`) or introduces a system actor.

| Actor ID | Name | Type | Maps To |
|---|---|---|---|
| ACT-01 | Merchant Admin | Human (Operator) | STK-003 |
| ACT-02 | Merchant Finance Operator | Human (Operator) | STK-001 |
| ACT-03 | Merchant Developer | Human (Operator) | STK-002 |
| ACT-05 | End Customer | Human (Indirect) | STK-013 |
| ACT-06 | Platform Compliance Officer | Human (Internal) | STK-010 |
| ACT-07 | Platform Support/Ops Engineer | Human (Internal) | STK-011 |
| ACT-08 | Acquirer/PSP System | External System | STK-005 |
| ACT-09 | AI Payment Assistant | System (AI) | Part 6 |
| ACT-10 | Orchestration Engine | System | Part 5 |
| ACT-11 | Scheduler/Cron (Dunning, Reconciliation Jobs) | System | Part 4/5 |
| ACT-12 | Bank Settlement File Source | External System | STK-005 |

---

## 3. PROC-01 — Operator Onboarding

### 3.1 Process Narrative

A new operator (merchant or platform operator) signs up, provides business and KYB information, uploads compliance documents, and is provisioned. Onboarding is evidence-collection and workflow only per BIZ-043 (Part 1) — the platform does not perform regulated KYC/KYB decisioning itself.

### UC-001: Register New Operator

- **Satisfies**: SCOPE-001
- **Primary Actor**: ACT-01 (Merchant Admin)
- **Preconditions**: Actor has a valid business email.
- **Main Flow**:
  1. ACT-01 submits company legal name, trade license number, country of incorporation (UAE at MVP), business email.
  2. System creates a new Operator aggregate in `Pending` status (see Part 3 for aggregate definition) with a unique operator ID.
  3. System sends email verification to ACT-01.
  4. ACT-01 verifies email; Operator status moves to `Active-Unverified` (can configure sandbox, cannot process live transactions).
  5. System provisions default ABAC roles (Admin, Finance Operator, Developer, Read-Only) with ACT-01 assigned as first Admin.
- **Alternate Flows**:
  - **AF-001a**: Trade license number fails basic format validation → inline validation error, no aggregate created.
  - **AF-001b**: Tenant subdomain already taken → system suggests alternatives.
- **Exception Flows**:
  - **EX-001a**: Email verification not completed within 72 hours → Tenant auto-transitions to `Expired-Unverified`; ACT-01 must restart registration.
- **Postconditions**: Tenant exists in `Active-Unverified` state; sandbox environment accessible; live processing blocked until KYB completion (UC-002).
- **Business Rules**: BR-001-1: A tenant cannot process a single live transaction until KYB status = `Approved` (UC-002). BR-001-2: Sandbox access is available immediately post email-verification to support merchant developer integration work in parallel with KYB (supports ACT-03/STK-002 need for fast integration start, per Part 1 §9 Persona "Rashid").

### UC-002: Submit & Track KYB Evidence

- **Satisfies**: BIZ-043, BIZ-040
- **Primary Actor**: ACT-01, secondary ACT-06 (Compliance Officer, internal reviewer path)
- **Preconditions**: Operator in `Active-Unverified` state (UC-001 complete).
- **Main Flow**:
  1. ACT-01 uploads trade license, Emirates ID/passport of authorized signatory, proof of business address, and bank account verification letter via the document upload flow (uses MinIO-backed storage, Part 4/9).
  2. System runs each uploaded document through the OCR/document pipeline (Qwen3-VL 8B vision model, Part 6) to extract structured fields (license number, expiry date, signatory name) for pre-fill and validation.
  3. System creates a `KybCase` in `Submitted` status and routes it either to (a) an integrated licensed KYB partner API for automated decisioning, or (b) an internal review queue for ACT-06 if no automated partner is configured for the tenant's jurisdiction/risk tier.
  4. ACT-06 (or partner system) reviews evidence and sets `KybCase` status to `Approved` or `Rejected` (with reason).
  5. On `Approved`, Operator status transitions to `Active-Verified`; live processing unblocked (subject to UC-003 acquirer connection).
- **Alternate Flows**:
  - **AF-002a**: Extracted OCR fields conflict with manually entered fields → flagged for ACT-06 manual reconciliation before approval.
  - **AF-002b**: Partner KYB API times out or errors → case automatically falls back to internal review queue (ACT-06) rather than blocking indefinitely.
- **Exception Flows**:
  - **EX-002a**: `KybCase` rejected → ACT-01 notified with reason category (not necessarily full partner rationale, to respect partner confidentiality terms) and may resubmit corrected evidence, creating a new `KybCase` version linked to the same Tenant.
- **Postconditions**: Operator KYB status recorded and immutably logged (BIZ-040); Operator either unblocked for live processing or remains restricted.
- **Business Rules**: BR-002-1: The platform itself never renders a KYB "decision" as a regulated act when a licensed partner is configured — the partner's decision is stored as-is; when no partner is configured, ACT-06's decision is an internal risk-acceptance decision by the platform operator, not a regulated KYB decision, and must be labeled as such in the audit record (ties to Part 1 §6.4 custody/licensing posture).

### UC-003: Connect First Acquirer/PSP

- See PROC-02 (§4, UC-010) — connecting an acquirer is treated as its own process because it is reused for adding subsequent acquirers, not just at onboarding.

---

## 4. PROC-02 — Acquirer/PSP Connection & Routing Configuration

### UC-010: Connect an Acquirer/PSP

- **Satisfies**: BIZ-010, SCOPE-002
- **Primary Actor**: ACT-01 or ACT-03 (Admin or Developer role, per ABAC)
- **Preconditions**: Operator status `Active-Verified` (UC-002 complete) for live mode; sandbox mode available pre-verification.
- **Main Flow**:
  1. Actor selects an acquirer/PSP from the supported connector catalog (Part 7 defines the Gateway Connector Framework and initial supported list).
  2. Actor supplies the credentials/configuration required by that connector's onboarding schema (API key, merchant ID, webhook secret, etc. — schema varies per connector, defined in Part 7).
  3. System validates credentials via a lightweight connectivity check call to the acquirer's sandbox/status endpoint.
  4. System creates a `MerchantAcquirerLink` aggregate in `Connected-Untested` status.
  5. Actor triggers a test transaction (small authorize+void) against the acquirer's sandbox.
  6. On success, `MerchantAcquirerLink` transitions to `Active`.
- **Alternate Flows**:
  - **AF-010a**: Credentials invalid → system reports specific validation error returned by the acquirer, does not create the link in `Active` state.
  - **AF-010b**: Actor connects a second (or third) acquirer for redundancy → each additional acquirer link is independent; routing rules (UC-011) determine priority among `Active` links.
- **Exception Flows**:
  - **EX-010a**: Acquirer sandbox unreachable during test transaction → link remains `Connected-Untested`; actor can retry or contact support (ACT-07).
- **Postconditions**: One or more `Active` `MerchantAcquirerLink`s exist for the operator; routing configuration (UC-011) can now reference them.
- **Business Rules**: BR-010-1: The operator cannot process live transactions with zero `Active` acquirer links. BR-010-2: Acquirer credentials are stored encrypted at rest and are never returned in full via any read API after initial save (Part 8, secrets handling).

### UC-011: Configure Routing Rules

- **Satisfies**: BIZ-010, BIZ-012, GOAL-002
- **Primary Actor**: ACT-01 (Admin) or ACT-02 (Finance Operator, if delegated permission)
- **Preconditions**: At least one `Active` `MerchantAcquirerLink` (UC-010).
- **Main Flow**:
  1. Actor defines a routing policy: static priority order (e.g., Acquirer A first, Acquirer B fallback), or rule-based (e.g., "route Visa/Mastercard UAE-issued cards to Acquirer A; route Amex to Acquirer B"; H3: success-rate-weighted dynamic routing per GOAL-009).
  2. Actor defines failover conditions: which decline codes/timeouts trigger a retry on the next acquirer in priority order, and the maximum number of retry hops per transaction (to bound latency and avoid excessive retries perceived as fraud probing).
  3. System validates the policy (e.g., no circular references, at least one terminal acquirer in every branch) and activates it.
- **Alternate Flows**:
  - **AF-011a**: Actor defines a currency-specific or amount-threshold-specific rule (e.g., route transactions over a configured threshold to a specific acquirer with better large-ticket rates) — supported as a rule condition type.
- **Exception Flows**:
  - **EX-011a**: Policy validation fails (e.g., a rule references a disconnected/inactive acquirer link) → policy save rejected with specific error; previous active policy remains in effect.
- **Postconditions**: An active `RoutingPolicy` exists and is used by the Orchestration Engine (Part 5) for every subsequent transaction.
- **Business Rules**: BR-011-1: Changing a `RoutingPolicy` takes effect only for new transactions initiated after the change; in-flight transactions continue under the policy version active at their initiation (immutability principle, ties to Part 3 event sourcing design). BR-011-2: Every routing policy change is audit-logged with actor, timestamp, before/after diff (BIZ-040, BIZ-042).

---

## 5. PROC-03 — Transaction Lifecycle

### UC-020: Authorize & Capture a Payment (Direct Checkout)

- **Satisfies**: BIZ-010, BIZ-012, BIZ-013, GOAL-001, GOAL-002
- **Primary Actor**: ACT-05 (End Customer, via merchant's checkout UI/API), system actor ACT-10 (Orchestration Engine)
- **Preconditions**: The operator has an active `RoutingPolicy` (UC-011); order/invoice or ad hoc charge request exists.
- **Main Flow**:
  1. ACT-05 submits payment details (via merchant's own checkout, hosted payment page, or payment link — see PROC-04) which the merchant's system (or the platform's hosted page) forwards to the platform's Payment Intent API.
  2. ACT-10 creates a `PaymentIntent` in `Created` status, evaluates the active `RoutingPolicy`, and selects the first eligible acquirer.
  3. ACT-10 sends an authorize request to ACT-08 (selected acquirer).
  4. On `Approved`, ACT-10 transitions `PaymentIntent` to `Authorized`, then — per tenant's capture mode configuration (auto-capture vs. manual) — either immediately requests capture from ACT-08 or waits for explicit capture (UC-021).
  5. ACT-10 emits domain events (`PaymentAuthorized`, `PaymentCaptured`) consumed by the reconciliation read-model (PROC-05) and analytics pipeline (PROC-08).
  6. Merchant/end customer receives a synchronous API response and, for asynchronous confirmation, a webhook (Part 10).
- **Alternate Flows**:
  - **AF-020a — Failover retry**: ACT-08 (primary acquirer) declines with a retryable decline code (per routing policy config, UC-011) → ACT-10 automatically attempts the next acquirer in priority order, up to the configured max retry hops, before returning a final decline to the merchant. This is the direct realization of BIZ-012/GOAL-002.
  - **AF-020b — 3-D Secure / step-up authentication required**: ACT-08 responds requiring additional customer authentication → ACT-10 relays the challenge to the merchant's checkout flow; on completion, flow resumes at step 4.
- **Exception Flows**:
  - **EX-020a**: All configured acquirers in the routing chain decline or time out → `PaymentIntent` transitions to `Failed`; end customer sees a final decline; event `PaymentFailedAllRoutes` is emitted (used by AI Assistant anomaly context, Part 6, and analytics, Part 8).
  - **EX-020b**: Acquirer returns an ambiguous/timeout response after having potentially processed the authorization (network partition scenario) → ACT-10 must perform a status-check/reconciliation call before retrying on a different acquirer, to avoid double-authorization (critical business rule, detailed further in Part 5 as an idempotency/reconciliation safeguard).
- **Postconditions**: `PaymentIntent` in a terminal or stable intermediate state (`Authorized`, `Captured`, `Failed`); all state transitions recorded as immutable domain events.
- **Business Rules**: BR-020-1: A `PaymentIntent` must never be captured twice for the same underlying acquirer authorization (idempotency, enforced via idempotency keys — Part 5/Part 10). BR-020-2: Retry/failover must respect a tenant-configured maximum latency budget so that failover does not create unacceptable checkout delay for the end customer (target values in Part 11, performance NFRs).

### UC-021: Manual Capture / Void

- **Satisfies**: BIZ-010
- **Primary Actor**: ACT-02 (Finance Operator) or ACT-03 (Developer, via API)
- **Preconditions**: `PaymentIntent` in `Authorized` status with tenant capture mode set to `Manual`.
- **Main Flow**: Actor triggers capture (full or partial amount) or void via dashboard/API; ACT-10 forwards to the acquirer that holds the authorization; `PaymentIntent` transitions accordingly.
- **Exception Flows**: **EX-021a**: Capture attempted after the acquirer's authorization validity window has expired → acquirer rejects; `PaymentIntent` transitions to `AuthorizationExpired`; actor must re-authorize if still desired.
- **Postconditions**: `PaymentIntent` reflects actual settled/void state.

### UC-022: Refund a Payment

- **Satisfies**: BIZ-010, BIZ-013 — see also PROC-07 (Dispute handling) for chargeback-driven refunds.
- **Primary Actor**: ACT-02 or ACT-03
- **Preconditions**: `PaymentIntent` in `Captured` status.
- **Main Flow**: Actor requests full or partial refund; ACT-10 routes the refund request to the *same* acquirer that captured the original payment (refunds are never rerouted to a different acquirer, since only the original acquirer holds the underlying settlement obligation); `PaymentIntent` (or a linked `RefundIntent`) transitions to `Refunded`/`PartiallyRefunded`.
- **Exception Flows**: **EX-022a**: Refund requested for an amount exceeding the remaining refundable balance (accounting for prior partial refunds) → rejected with a clear validation error before any call reaches the acquirer.
- **Postconditions**: Refund event recorded; reconciliation read-model updated (PROC-05); relevant for settlement matching.
- **Business Rules**: BR-022-1: Since the platform holds no custody (Part 1 §6), a refund is *always* an instruction to the acquirer to return funds from the merchant's own settlement flow — the platform never fronts refund money itself.

---

## 6. PROC-04 — Invoicing, Payment Links & Subscription Billing

### UC-030: Create and Send an Invoice

- **Satisfies**: BIZ-014
- **Primary Actor**: ACT-02
- **Preconditions**: Tenant `Active-Verified`; at least one active acquirer link.
- **Main Flow**:
  1. ACT-02 creates an `Invoice` with line items, due date, currency, and recipient (end customer) contact details.
  2. System generates a hosted, branded payment page URL and (optionally) sends it via email/SMS to ACT-05.
  3. ACT-05 opens the link, completes checkout, which internally invokes UC-020 (Authorize & Capture), linked back to the `Invoice`.
  4. On successful payment, `Invoice` transitions to `Paid`; on partial payment (if allowed by tenant config), transitions to `PartiallyPaid`.
- **Alternate Flows**: **AF-030a**: ACT-02 schedules a recurring invoice (weekly/monthly) → creates a `RecurringInvoiceSchedule` which spawns individual `Invoice` instances per period, feeding into PROC subscription logic (UC-031) if configured as a subscription rather than ad hoc recurring invoice.
- **Exception Flows**: **EX-030a**: Invoice due date passes unpaid → transitions to `Overdue`; triggers configured reminder notifications (Part 4, Notification Service).
- **Postconditions**: Invoice lifecycle fully tracked and linked to underlying `PaymentIntent`(s) for reconciliation.

### UC-031: Create a Subscription Plan & Handle Renewal

- **Satisfies**: BIZ-015
- **Primary Actor**: ACT-01/ACT-02 (plan setup), ACT-11 Scheduler (renewal execution)
- **Preconditions**: Tenant `Active-Verified`.
- **Main Flow**:
  1. Actor defines a `SubscriptionPlan` (price, billing interval, trial period rules).
  2. ACT-05 subscribes (via checkout, storing a reusable payment method token per acquirer's tokenization capability — never raw card data touches platform storage, Part 8).
  3. On each billing cycle, ACT-11 triggers a renewal `PaymentIntent` using the stored payment method token, following the same routing/failover logic as UC-020.
  4. On success, subscription period extends; on failure, dunning flow (AF-031a) begins.
- **Alternate Flows — AF-031a (Dunning)**: Renewal declines → system retries per a configured dunning schedule (e.g., retry at day 1, 3, 7) potentially across different acquirers per routing policy; if all retries exhausted, subscription transitions to `PastDue` then `Cancelled` per tenant policy; ACT-05 notified at each step (Part 4, Notification Service).
- **Exception Flows**: **EX-031a**: Payment method token itself is invalidated by the acquirer/scheme (e.g., card expired, scheme token update available) → system attempts scheme-provided account-updater refresh if the connector supports it (Part 7), else flags subscription for customer payment-method update.
- **Postconditions**: Subscription lifecycle state accurately reflects billing history; each renewal attempt is a fully traceable `PaymentIntent` history.
- **Business Rules**: BR-031-1: Stored payment method tokens are always acquirer-issued tokens (network/PSP tokenization), never platform-generated raw-PAN storage — ties to Part 8 (PCI-scope minimization) and Part 1 no-custody-adjacent principle of minimizing sensitive data the platform itself must protect.

---

## 7. PROC-05 — Settlement & Reconciliation

### UC-040: Ingest Acquirer Settlement File / Webhook

- **Satisfies**: BIZ-013, GOAL-003
- **Primary Actor**: ACT-12 (Bank/Acquirer Settlement File Source), ACT-11 (Scheduler for polling-based connectors)
- **Preconditions**: Active `MerchantAcquirerLink`; connector defines settlement ingestion mechanism (real-time webhook, polling API, or SFTP file drop — Part 7).
- **Main Flow**:
  1. System receives/pulls settlement data (per-transaction settlement confirmation, batch settlement file, or summary report depending on connector capability).
  2. System normalizes the acquirer-specific settlement format into the platform's internal `SettlementRecord` schema (Part 9).
  3. System matches each `SettlementRecord` against the corresponding `PaymentIntent` by acquirer transaction reference.
  4. Matched records update the `PaymentIntent`'s settlement status (`Settled`, amount, settlement date, fees deducted by acquirer if reported).
- **Alternate Flows**: **AF-040a**: Settlement record references a transaction reference not found in the platform (e.g., manual/legacy transaction processed outside the platform) → recorded as an `UnmatchedSettlementRecord` surfaced to ACT-02 for manual review/annotation, rather than silently dropped.
- **Exception Flows**: **EX-040a**: Settlement file fails checksum/format validation → file quarantined, ACT-07 (Support/Ops) alerted, ingestion retried after correction.
- **Postconditions**: Reconciliation read-model (ClickHouse-backed analytics + Postgres-backed transactional reconciliation state, Part 9) reflects up-to-date settlement status.
- **Business Rules**: BR-040-1: Settlement ingestion is idempotent — re-ingesting the same file/webhook must not duplicate `SettlementRecord`s (dedup key per Part 9 schema design).

### UC-041: Review Reconciliation Exceptions

- **Satisfies**: BIZ-013, GOAL-003
- **Primary Actor**: ACT-02
- **Preconditions**: One or more `UnmatchedSettlementRecord`s or amount-mismatch exceptions exist.
- **Main Flow**: ACT-02 opens the reconciliation exceptions view; for each exception, either (a) manually links it to an existing `PaymentIntent`, (b) flags it as a genuine discrepancy requiring acquirer support-ticket follow-up, or (c) asks the AI Assistant (PROC-06, UC-050) to investigate and suggest a likely match based on amount/date/customer proximity heuristics.
- **Postconditions**: Exception queue shrinks over time; all resolutions are audit-logged (who resolved, how, when).
- **Business Rules**: BR-041-1: AI Assistant may *suggest* a match (with cited evidence) but a human actor must confirm before the system treats it as reconciled — AI does not auto-resolve financial exceptions without human-in-the-loop confirmation (ties to Part 6 AI guardrails and Part 1 BIZ-023 citability requirement).

---

## 8. PROC-06 — AI Assistant Interaction (Cross-Cutting)

### UC-050: Ask the AI Payment Assistant an Operational Question

- **Satisfies**: BIZ-020, BIZ-021, BIZ-023
- **Primary Actor**: ACT-01, ACT-02, or ACT-03 (any authenticated tenant user with appropriate data-access scope)
- **Preconditions**: Tenant has sufficient transaction/reconciliation history for the question to be answerable; AI Assistant service available (Ollama-hosted models up, per Part 6/Part 11 availability targets).
- **Main Flow**:
  1. Actor asks a natural-language question in the Assistant panel (e.g., "Why did our authorization rate drop yesterday?").
  2. System (Part 6 RAG pipeline) embeds the query (BGE-M3), retrieves relevant tenant-scoped documents/events (transaction aggregates, decline-code summaries, prior similar Q&A), reranks results, and constructs a grounded prompt for the Qwen3 32B model.
  3. Model produces an answer that includes specific cited references (transaction IDs, date ranges, decline code breakdowns) rather than an unattributed narrative summary.
  4. Actor sees the answer plus expandable citations linking to the underlying data records.
- **Alternate Flows**: **AF-050a**: Actor uploads a document (e.g., a scanned bank advice) as part of the question → Qwen3-VL 8B vision pipeline extracts structured content first, which is then included in the RAG context.
- **Exception Flows**: **EX-050a**: Retrieval finds insufficient grounding data to answer confidently → Assistant explicitly states it cannot answer reliably rather than fabricating a plausible-sounding but ungrounded answer (critical guardrail, detailed in Part 6).
- **Postconditions**: Question and answer (with citations) logged for audit and for future retrieval-quality evaluation (Part 6 evaluation harness).
- **Business Rules**: BR-050-1: The Assistant's retrieval is bounded to the operator's own data.

---

## 9. PROC-07 — Dispute, Refund & Chargeback Handling

### UC-060: Record and Track a Chargeback

- **Satisfies**: BIZ-013, BIZ-040
- **Primary Actor**: ACT-08 (Acquirer, via webhook/notification), ACT-02 (handling)
- **Preconditions**: A `Captured` `PaymentIntent` exists that the chargeback references.
- **Main Flow**:
  1. ACT-08 notifies the platform of a chargeback (via connector-specific webhook, Part 7).
  2. System creates a `ChargebackCase` linked to the `PaymentIntent`, in `Received` status, and notifies ACT-02.
  3. ACT-02 reviews evidence requirements (per acquirer/scheme) and submits representment evidence (if disputing) via the platform, which forwards it to ACT-08's dispute API where supported.
  4. `ChargebackCase` resolves to `Won`, `Lost`, or `Accepted` (merchant chose not to dispute) based on ACT-08's final determination, recorded when received.
- **Postconditions**: Chargeback outcome is reflected in reconciliation (funds impact is reported by the acquirer/scheme, not custodied by the platform — Part 1 §6) and in analytics (chargeback-rate monitoring, relevant to scheme compliance thresholds).
- **Business Rules**: BR-060-1: Chargeback-rate analytics must be available per acquirer/card scheme because sustained high chargeback ratios can trigger scheme monitoring programs — this is a merchant risk the platform must help surface proactively (ties to GOAL-010 proactive AI operations, H3).

---

## 10. PROC-08 — Reporting & Analytics

### UC-070: View Unified Transaction Dashboard

- **Satisfies**: BIZ-013, SCOPE-007
- **Primary Actor**: ACT-01, ACT-02
- **Main Flow**: Actor views authorization rates, decline-reason breakdowns, settlement status, and revenue-recovered-via-failover metrics, aggregated across all connected acquirers, backed by ClickHouse analytics store (Part 9).
- **Postconditions**: N/A (read-only); heavy queries served from ClickHouse rather than the transactional Postgres store to avoid impacting orchestration performance (architecture rationale detailed in Part 4/9).

### UC-071: Export Reconciliation Report

- **Satisfies**: BIZ-013, GOAL-003
- **Primary Actor**: ACT-02
- **Main Flow**: Actor requests a reconciliation report for a date range/acquirer; system generates a downloadable export (CSV/PDF) reflecting matched, unmatched, and exception records as of generation time.
- **Business Rules**: BR-071-1: Exported reports are themselves stored (MinIO) and audit-logged as a compliance evidence artifact (BIZ-040), since finance teams often need to prove what a report showed at a point in time (audit reproducibility).

---

## 12. Use Case Traceability Matrix (Summary)

| Use Case | Business Requirement(s) | Bounded Context (preview, Part 3) |
|---|---|---|
| UC-001 Register Operator | SCOPE-001 | Operator Management |
| UC-002 KYB Evidence | BIZ-043, BIZ-040 | Operator Management / Compliance |
| UC-010 Connect Acquirer | BIZ-010 | Gateway Connector Framework |
| UC-011 Routing Rules | BIZ-010, BIZ-012 | Payment Orchestration |
| UC-020 Authorize & Capture | BIZ-010, BIZ-012, BIZ-013 | Payment Orchestration |
| UC-021 Manual Capture/Void | BIZ-010 | Payment Orchestration |
| UC-022 Refund | BIZ-010, BIZ-013 | Payment Orchestration |
| UC-030 Invoice | BIZ-014 | Invoice Service |
| UC-031 Subscription | BIZ-015 | Subscription Billing |
| UC-040 Settlement Ingestion | BIZ-013 | Reconciliation / Settlement |
| UC-041 Reconciliation Exceptions | BIZ-013 | Reconciliation / Settlement |
| UC-050 AI Assistant Q&A | BIZ-020, BIZ-021, BIZ-023 | AI Payment Assistant (RAG) |
| UC-060 Chargeback | BIZ-013, BIZ-040 | Dispute Management |
| UC-070/071 Reporting | BIZ-013 | Analytics & Reporting |

*(Full matrix including NFR cross-references will be consolidated in Part 12, Appendices.)*

---

## 13. Open Items Carried Forward

- **OQ-005**: Exact dunning retry schedule defaults (UC-031) need Product sign-off — placeholder values used here (day 1/3/7) pending finalization.
- **OQ-006**: Whether AI-suggested reconciliation matches (UC-041) require a secondary approver for amounts above a configurable threshold — recommend yes, to be confirmed with STK-010 (Compliance) before Part 8 finalizes the control.

---

*End of Part 2. Proceed to Part 3: Domain-Driven Design & Bounded Contexts.*
