# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 5 of 12:** Payment Orchestration Engine — Deep Dive
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 5 of 12 — Payment Orchestration Engine |
| Depends On | Part 3 (BC-05 domain model, AGG-01/AGG-02), Part 4 (SVC-05 service boundary) |
| Feeds Into | Part 7 (Connector Framework, the ACL this engine calls), Part 9 (event store schema), Part 10 (gRPC contracts), Part 11 (performance/latency NFRs, TDD standards) |
| Scope of This Part | Full internal design of `orchestration-service`: command handling, state machine, routing algorithm, failover mechanics, idempotency guarantees, concurrency control, and the marketplace-split addendum. |

---

## 1. Command Catalog

Every mutation to `PaymentIntent` (AGG-01, Part 3) or `RoutingPolicy` (AGG-02) is expressed as one of the following commands. Each command is validated against current aggregate state (loaded by folding its event stream) before producing zero or more events.

| Command | Preconditions | Produces (on success) | Produces (on rejection) |
|---|---|---|---|
| `CreatePaymentIntent` | Valid `Money`, valid tenant, idempotency key not previously used for a *different* payload | `PaymentIntentCreated` (EVT-01) | Command rejected synchronously (no event) — duplicate idempotency key with identical payload returns the original result instead of erroring (idempotent replay, not an error) |
| `AuthorizePaymentIntent` | `PaymentIntent` in `Created` or `Failed` (mid-retry) state; active `RoutingPolicy` exists | `PaymentAuthorizationAttempted` (EVT-02), then `PaymentAuthorized` (EVT-03) or `PaymentFailed` (EVT-06) | N/A — always produces at least an attempt event; "rejection" here means a failed attempt, not a no-op |
| `CapturePaymentIntent` | `PaymentIntent` in `Authorized` state; requested amount ≤ remaining authorized amount (INV-01) | `PaymentCaptured` (EVT-04) or `PaymentPartiallyCaptured` (EVT-05) | Command rejected synchronously if amount exceeds authorized remainder |
| `VoidPaymentIntent` | `PaymentIntent` in `Authorized` state, not yet captured | `PaymentVoided` (EVT-08) | Rejected if already captured |
| `RefundPaymentIntent` | `PaymentIntent` in `Captured`/`PartiallyCaptured` state; amount ≤ remaining refundable balance (BR-022-1) | `PaymentRefunded` (EVT-09) or `PaymentPartiallyRefunded` (EVT-10) | Rejected synchronously if amount exceeds refundable balance |
| `ActivateRoutingPolicy` | New policy passes validation (Part 3 INV-05: no circular refs, all referenced acquirer links `Active`) | `RoutingPolicyActivated` (EVT-11), previous policy's `RoutingPolicyDeactivated` (EVT-12) | Rejected synchronously with validation error |

**Design rule (CMD-001)**: Commands that can be meaningfully retried by a caller without side effects (e.g., `CreatePaymentIntent`) are idempotent by construction using a caller-supplied `IdempotencyKey`. Commands that represent "try to move money" (`AuthorizePaymentIntent`) are *not* silently idempotent in the same way — they always attempt the next routing hop — but the underlying acquirer call within them is protected by a separate acquirer-level idempotency key (§4) to prevent duplicate authorizations at the external system.

---

## 2. The `PaymentIntent` State Machine

### 2.1 Full State Diagram (textual)

```
                 ┌──────────┐
                 │ Created  │
                 └────┬─────┘
                      │ AuthorizePaymentIntent
                      ▼
              ┌───────────────┐
        ┌────►│  Authorizing  │◄────┐  (re-entered on each retry hop)
        │     └───────┬───────┘     │
        │             │             │
        │   decline (retryable,     │ retry on next acquirer
        │   hops remain)            │ (AF-020a)
        │             └─────────────┘
        │             │
        │   decline (non-retryable, or hops exhausted)
        │             ▼
        │     ┌───────────────────┐
        │     │ Failed /           │  (Failed = single-hop terminal for that
        │     │ FailedAllRoutes    │   attempt sequence if no more hops)
        │     └───────────────────┘
        │
        │  success
        ▼
  ┌─────────────┐   CapturePaymentIntent (auto or manual)   ┌───────────────┐
  │  Authorized │ ─────────────────────────────────────────►│   Capturing   │
  └──────┬──────┘                                            └───────┬───────┘
         │ VoidPaymentIntent                                          │ success
         ▼                                                            ▼
  ┌─────────────┐                                             ┌───────────────┐
  │   Voided    │                                             │   Captured /   │
  └─────────────┘                                             │ PartiallyCap.  │
         ▲                                                     └───────┬────────┘
         │ authorization window expires (no capture attempted)         │ RefundPaymentIntent
  ┌──────┴──────────────┐                                              ▼
  │ AuthorizationExpired │                                     ┌────────────────┐
  └──────────────────────┘                                     │ Refunded /      │
                                                                 │ PartiallyRefund.│
                                                                 └────────────────┘
```

### 2.2 State Transition Table (authoritative)

| From | Event | To | Guard |
|---|---|---|---|
| `Created` | `PaymentAuthorizationAttempted` | `Authorizing` | Active `RoutingPolicy` resolves ≥1 eligible acquirer |
| `Authorizing` | `PaymentAuthorized` | `Authorized` | Acquirer approved |
| `Authorizing` | `PaymentFailed` (retryable, hops remain) | `Authorizing` (re-attempt next hop) | Decline reason in tenant's configured retryable set (UC-011) AND hop count < max |
| `Authorizing` | `PaymentFailedAllRoutes` | `Failed` (terminal) | Decline non-retryable OR hop count = max |
| `Authorized` | `PaymentCaptured` / `PaymentPartiallyCaptured` | `Captured` / `PartiallyCaptured` | INV-01 amount check |
| `Authorized` | `PaymentVoided` | `Voided` | Not yet captured |
| `Authorized` (no action within acquirer's auth validity window) | *(no explicit command — a scheduled check emits this)* `AuthorizationExpired` | `AuthorizationExpired` | Time-based, checked by JOB (§7) |
| `Captured` / `PartiallyCaptured` | `PaymentRefunded` / `PaymentPartiallyRefunded` | `Refunded` / `PartiallyRefunded` | BR-022-1 balance check, INV-03 same-acquirer check |

---

## 3. Routing Algorithm

### 3.1 Inputs

At `AuthorizePaymentIntent` time, the engine gathers:
1. The tenant's currently `Active` `RoutingPolicy` version (Part 3 INV-05 — always the version active *at this moment*, and that exact version ID is recorded on the `PaymentAuthorizationAttempted` event for later audit).
2. The set of `Active` `MerchantAcquirerLink`s.
3. Transaction attributes relevant to routing conditions: card scheme (Visa/Mastercard/Amex/mada, as reported at tokenization/entry time), currency, amount, and (if `risk-service`, SVC-11, is enabled for the tenant per OQ-009) a risk score.
4. (H3 only, GOAL-009) Historical authorization-rate statistics per acquirer/card-scheme/currency combination, sourced from `analytics-service` read models, for success-rate-weighted dynamic routing.

### 3.2 Algorithm (MVP — Static/Rule-Based)

```
function select_route(intent, policy, links, attempted_hops):
    candidates = policy.rules
        .filter(rule -> rule.matches(intent.card_scheme, intent.currency, intent.amount))
        .map(rule -> rule.acquirer_link_id)
        .filter(link_id -> link_id in links.active_ids())
        .filter(link_id -> link_id not in attempted_hops)   // never retry the same acquirer twice for one intent
        .order_by(rule.priority)

    if candidates.is_empty():
        return NoEligibleRoute   // -> PaymentFailedAllRoutes

    return candidates.first()
```

### 3.3 Algorithm (H3 — Success-Rate-Weighted Dynamic Routing, GOAL-009)

Extends 3.2 by re-weighting `candidates` using a rolling window (e.g., trailing 1 hour, tenant-configurable) of authorization-rate statistics per acquirer/scheme/currency, subject to a **minimum-sample-size floor** (to avoid a single recent decline skewing routing for a low-volume combination) and a **maximum deviation cap** from the tenant's explicitly configured static priority (to prevent the dynamic layer from silently overriding a tenant's deliberate business preference, e.g., a negotiated-cost priority — dynamic routing optimizes *within* tenant-set boundaries, not around them). Full statistical design (window size defaults, cap defaults, confidence thresholds) is deferred to an H3 design spike, explicitly flagged as **not required for MVP acceptance** (Part 11).

### 3.4 Failover Retry Mechanics

- **RTY-001**: On a retryable decline, the engine immediately attempts the next candidate — there is no artificial delay between hops within a single checkout attempt (retries are about trying a *different* acquirer, not waiting out a transient issue on the same one).
- **RTY-002**: The maximum number of hops is tenant-configurable (UC-011) but the engine enforces a hard platform-wide ceiling (default 3, configurable per deployment, not per tenant, as a cost/latency circuit breaker) regardless of tenant configuration, to bound worst-case checkout latency.
- **RTY-003**: Each hop's `PaymentAuthorizationAttempted` event records the acquirer attempted, the raw acquirer response (via BC-04's normalized `DeclineReason`), and elapsed latency for that hop — this is the ground truth `ai-assistant-service` and `analytics-service` use to answer "why did this decline" questions (UC-050) and to compute GOAL-002's revenue-recovery metric.

---

## 4. Idempotency & Concurrency Control

### 4.1 Two Distinct Idempotency Layers

1. **Caller-facing idempotency** (`CreatePaymentIntent`): keyed on tenant-scoped `IdempotencyKey` supplied by the merchant's integration. A retried request with the same key and same payload returns the original `PaymentIntent`'s current state rather than creating a duplicate or erroring. A retried request with the same key but a *different* payload is rejected with a conflict error (protects against integration bugs silently creating divergent state under one key).
2. **Acquirer-facing idempotency** (within `AuthorizePaymentIntent`'s call to `connector-gateway`): a separate, internally generated idempotency token per routing attempt, passed through BC-04's ACL to the specific acquirer's own idempotency mechanism (where supported — Part 7 documents per-connector support level), specifically to guard against EX-020b's network-partition double-authorization risk. Where an acquirer does not support idempotency keys natively, the connector adapter must perform a pre-flight status-check call before retrying the same acquirer (never applicable here anyway, since RTY-001 never retries the same acquirer twice for one intent — but the same status-check discipline applies if a client-side timeout occurs and the *caller* retries `AuthorizePaymentIntent` itself).

### 4.2 Concurrency Control

- **CONC-001**: `PaymentIntent` aggregate mutations use optimistic concurrency control at the event-store level (expected version check on append) — two concurrent commands against the same `PaymentIntent` (e.g., a manual capture request racing a webhook-driven auto-capture) will have one succeed and one rejected-and-retried-after-reload, never both silently applied.
- **CONC-002**: The idempotency cache (Redis, Part 4 SVC-05 datastore) is used as a fast-path duplicate-request check *before* touching the event store, but the event store's optimistic concurrency check remains the ultimate source of truth — Redis is a performance optimization, not the correctness mechanism (defense in depth: Redis unavailability degrades performance, not correctness).

---

## 5. Sequence Walkthrough — Authorize with One Failover Hop

*(Textual sequence diagram; formal UML-ready sequence diagrams to be produced as a diagram appendix in Part 12.)*

1. Merchant checkout (via SDK) → API Gateway → `orchestration-service`: `CreatePaymentIntent(amount, currency, idempotency_key)`.
2. `orchestration-service`: validates, appends `PaymentIntentCreated`, returns `payment_intent_id` + status `Created`.
3. Merchant checkout → `orchestration-service`: `AuthorizePaymentIntent(payment_intent_id, card_token)`.
4. `orchestration-service`: loads `RoutingPolicy`, selects Acquirer A (priority 1), appends `PaymentAuthorizationAttempted{acquirer=A}`.
5. `orchestration-service` → `connector-gateway` (gRPC, synchronous): `Authorize(acquirer=A, ...)`.
6. `connector-gateway` → Acquirer A (external HTTPS): authorize call.
7. Acquirer A responds: `Declined, reason=INSUFFICIENT_FUNDS` (mapped by BC-04 ACL to normalized `DeclineReason::InsufficientFunds`, tenant-configured as retryable).
8. `orchestration-service`: appends `PaymentFailed{acquirer=A, reason=InsufficientFunds}`; RTY-001 → immediately selects Acquirer B (priority 2, not yet attempted); appends `PaymentAuthorizationAttempted{acquirer=B}`.
9. `orchestration-service` → `connector-gateway` → Acquirer B: authorize call.
10. Acquirer B responds: `Approved`.
11. `orchestration-service`: appends `PaymentAuthorized{acquirer=B}`; publishes EVT-03 to NATS JetStream.
12. `orchestration-service` → merchant checkout: synchronous response `Authorized` (total elapsed time = hop 1 latency + hop 2 latency, within the tenant's configured latency budget per BR-020-2 — enforced by RTY-002's hard hop ceiling).
13. Async: `invoice-service`, `analytics-service`, `ai-assistant-service` (indexing), `notification-service` each independently consume EVT-03 from their own durable NATS consumer.

---

## 6. Marketplace-Split Addendum (BC-16 Integration, H2)

When a `PaymentIntent` is flagged (at `CreatePaymentIntent` time, via a `split_configuration_id` reference) as a marketplace transaction:

- **MKT-SPLIT-001**: Step 5 above (`connector-gateway` authorize call) additionally carries the active `SplitConfiguration` (from `marketplace-service`, SVC-16), which `connector-gateway`'s ACL translates into whatever the licensed split-disbursement partner's own API expects (this may mean the "acquirer" in this flow is actually the licensed partner's payment API, not a traditional card acquirer directly — Part 7 documents this per-partner).
- **MKT-SPLIT-002**: Per Part 3 INV-09 and Part 2 EX-080a, if the licensed partner rejects the split configuration at authorization time, the engine does **not** fall back to processing the payment as a non-split, platform-held transaction — it either (a) fails the transaction outright, or (b) processes it as fully non-split *only if* the tenant has explicitly pre-configured that fallback behavior for their marketplace integration, making the fallback an explicit tenant choice rather than an implicit platform default (protects the no-custody boundary, Part 1 §6, from being silently violated under a "just make it work" implementation shortcut).

---

## 7. Scheduled Consistency Jobs (Owned by `orchestration-service`)

- **JOB-007**: Authorization-expiry sweep — periodically scans `Authorized` `PaymentIntent`s whose acquirer authorization validity window (acquirer-specific, from BC-04 connector metadata) has passed without capture, and transitions them to `AuthorizationExpired` (Part 3 state machine, §2.1 above).
- **JOB-008**: Stuck-`Authorizing` reconciliation — detects `PaymentIntent`s that have remained in `Authorizing` beyond an anomalous duration (indicating a lost/never-received acquirer response) and triggers a status-check call to the relevant acquirer (where supported) rather than leaving the intent in permanent limbo; this is the systematic version of the ad hoc EX-020b safeguard.

---

## 8. Non-Functional Requirements Specific to This Service (Preview — Full NFRs in Part 11)

- **NFR-ORC-001**: p99 latency for a single-hop authorization (gateway-in to gateway-out) must not exceed a target to be finalized in Part 11, but must be materially tighter than the sum of all acquirer network round-trips would suggest is "free," since checkout abandonment is latency-sensitive.
- **NFR-ORC-002**: `orchestration-service` must remain available (accepting `CreatePaymentIntent`/`AuthorizePaymentIntent`) even if `analytics-service`, `notification-service`, or `ai-assistant-service` are degraded or down — these are async NATS consumers, not synchronous dependencies of the checkout path (structural resilience, not just a target).
- **NFR-ORC-003**: Event store writes (aggregate append) must be durable (committed to Postgres, not just in-memory) before the engine considers a state transition final and returns a success response — no "eventually durable" behavior on the write path for money-movement events.

---

## 9. Traceability

| Requirement | Realized By |
|---|---|
| BIZ-010 (configurable routing) | §3.2 algorithm consuming `RoutingPolicy` |
| BIZ-012 / GOAL-002 (failover) | §3.4, §5 walkthrough |
| BR-020-1 (no double capture) | §1 `CapturePaymentIntent` guard, INV-01 |
| BR-020-2 (latency budget) | §3.4 RTY-002, §8 NFR-ORC-001 |
| EX-020b (double-auth on network partition) | §4.1 acquirer-facing idempotency, §7 JOB-008 |
| BR-022-1 / INV-03 (refund same acquirer) | §1 `RefundPaymentIntent` guard |
| EX-080a / INV-09 (no custody fallback on split failure) | §6 MKT-SPLIT-002 |
| GOAL-009 (H3 smart routing) | §3.3 |

---

## 10. Open Items Carried Forward

- **OQ-011**: Finalize default and configurable-range values for RTY-002's hard hop ceiling — placeholder "3" used above pending a latency-budget modeling exercise in Part 11.
- **OQ-012**: Confirm which MVP acquirer partners support native idempotency tokens (§4.1) vs. require status-check-before-retry — depends on OQ-003 (Part 1) acquirer shortlist; must be resolved before Part 7 finalizes per-connector capability flags.

---

*End of Part 5. Proceed to Part 6: AI Payment Assistant & RAG Architecture.*
