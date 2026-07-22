# Comprehensive Platform Analysis
## Payment Orchestra — Architect's Deep Dive

> **Analyst Perspective:** Payment Gateway Router / Service Orchestrator architect who has built and operated multi-acquirer payment platforms at scale.  
> **Analysis Date:** July 22, 2026  
> **Scope:** Full backend docs (21 files), SRS (12 parts), feature spec, frontend/ios/android/landing-page architecture  
> **Guiding Principle:** "BYOK — merchants bring their own keys. One integration, connect to any payment gateway."

---

# TABLE OF CONTENTS

1. [Executive Summary](#1-executive-summary)
2. [Gap Analysis](#2-gap-analysis)
3. [Design Patterns Analysis](#3-design-patterns-analysis)
4. [Over-Engineering Analysis](#4-over-engineering-analysis)
5. [Payment Router Perspective](#5-payment-router-perspective)
6. [BYOK Model Deep Dive](#6-byok-model-deep-dive)
7. [Service-by-Service Assessment](#7-service-by-service-assessment)
8. [Critical Recommendations](#8-critical-recommendations)
9. [Revised Architecture Proposal](#9-revised-architecture-proposal)

---

## 1. Executive Summary

The documentation is **impressively thorough** — 21 backend service files plus 12-part SRS covering vision, architecture, security, compliance, database, APIs, testing, and deployment. The team has clearly done their homework on DDD, event sourcing, and modern Rust patterns.

However, the documentation reveals several critical issues:

### Core Problems

| # | Problem | Severity | Effort to Fix |
|---|---|---|---|
| P1 | **No dedicated MerchantAcquirerLink service** — the CORE entity of the BYOK model is referenced everywhere but never owned by any service | 🔴 Critical | Medium |
| P2 | **No webhook delivery service** for merchant outbound notifications — merchants cannot programmatically integrate without this | 🔴 Critical | Medium |
| P3 | **No sandbox/test environment** — merchants need to test before going live | 🔴 Critical | Medium |
| P4 | **No 3D Secure handling** — mandatory for card payments in UAE/Europe | 🔴 Critical | Large |
| P5 | **No circuit breaker pattern per acquirer** — needed to protect against acquirer outages | 🟠 High | Small |
| P6 | **18 microservices is too many** — splintered architecture adds operational complexity without clear benefit | 🟠 High | Large |
| P7 | **Protobuf-only external API** — unusual choice that adds friction for merchant adoption against Stripe/Adyen's REST JSON | 🟠 High | Medium |
| P8 | **No merchant SDK/client library specs** — merchants cannot integrate easily | 🟡 Medium | Medium |
| P9 | **No A/B testing / canary routing** — merchants need gradual rollout for new acquirers | 🟡 Medium | Medium |
| P10 | **Missing subscription pause/proration flow** — acknowledged but not fully detailed | 🟡 Medium | Small |

---

## 2. Gap Analysis

### 2.1 CRITICAL GAPS — Must Fix Before Launch

#### GAP-01: No MerchantAcquirerLink Service (BYOK Core)

**The Problem:** `MerchantAcquirerLink` is the MOST important domain entity in a BYOK payment orchestrator. It represents a merchant's connection to a specific payment gateway with their own credentials. Yet it has:

- ❌ No dedicated service
- ❌ No aggregate/entity definition (just referenced by ID)
- ❌ No command handlers (Create, Update, RotateCredentials, Disable, Enable, TestConnection)
- ❌ No repository interface
- ❌ No lifecycle management
- ❌ No credential validation flow
- ❌ No health monitoring per link

**Impact:** Merchants cannot onboard their own gateway credentials. The platform cannot validate that a merchant's API keys are working. There's no way to know if a credential has expired or been revoked.

**Required:** A new `merchant-acquirer-link-service` (or fold into connector-gateway) with:

```rust
// Proposed aggregate
pub struct MerchantAcquirerLink {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,           // 'network_international' | 'checkout_com' | 'telr'
    pub profile_name: String,           // user-friendly: "Production NI Gateway"
    pub environment: LinkEnvironment,   // Sandbox | Production
    pub credentials: EncryptedCredentials,
    pub status: LinkStatus,             // Active | Disabled | Testing | CredentialsExpired
    pub health_status: LinkHealth,      // Healthy | Degraded | Unreachable
    pub last_tested_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

pub enum LinkEnvironment { Sandbox, Production }
pub enum LinkStatus { Active, Disabled, Testing, CredentialsExpired }
pub enum LinkHealth { Healthy, Degraded, Unreachable, Unknown }
```

**Commands:**
- `CreateMerchantAcquirerLink` — with credential validation
- `TestMerchantAcquirerConnection` — validate credentials work
- `RotateMerchantAcquirerCredentials` — update without downtime
- `DisableMerchantAcquirerLink` — stop routing to this link
- `EnableMerchantAcquirerLink` — resume routing
- `ReportCredentialExpiry` — from health check job

#### GAP-02: No Webhook Delivery Service (Merchant Integration)

**The Problem:** The documentation references "webhook delivery" in cross-cutting (section 35 of 19-infrastructure-cross-cutting.md) but it's treated as an afterthought. **Webhooks are THE primary integration mechanism for payment orchestrator merchants.**

Every major payment platform (Stripe, Adyen, Braintree) provides:
- Webhook event types (documented here ✓)
- Webhook signing with HMAC (documented here ✓)
- **Webhook retry with exponential backoff** (partially documented)
- **Webhook delivery dashboard** (NOT documented)
- **Webhook testing console** (NOT documented — merchants need to simulate events)
- **Webhook endpoint health monitoring** (NOT documented)

**Missing Webhook Features:**
1. ❌ Webhook testing/simulation sandbox — merchants need to verify their webhook handler before going live
2. ❌ Webhook delivery dashboard — see delivery attempts, retries, failures per endpoint
3. ❌ Webhook endpoint health — alert if merchant endpoint is consistently failing
4. ❌ Webhook retry configuration — per-endpoint retry policy
5. ❌ Webhook event filtering — subscribe to specific event types only
6. ❌ Webhook schema versioning per endpoint — merchant chooses schema version

#### GAP-03: No 3D Secure Handling

**The Problem:** 3D Secure (3DS) is **mandatory** for card payments in the UAE (Central Bank regulations) and Europe (PSD2 SCA). The documentation doesn't mention 3DS anywhere.

**Required Flows:**
1. Check if card/enrollment requires 3DS (via acquirer or 3DS provider)
2. Redirect customer to issuer's 3DS page (or embedded challenge)
3. Receive 3DS result (via return URL or webhook)
4. Only authorize after successful 3DS authentication
5. Handle frictionless vs challenge flows
6. Support exemption requests (low-value, merchant-initiated, etc.)
7. Store 3DS authentication results for liability shift

**Architecture Impact:**
- `orchestration-service` needs 3DS status in PaymentIntent state machine
- `connector-gateway` needs 3DS capability per acquirer
- API Gateway needs 3DS redirect URL handling
- Hosted checkout page (`payment-link-service`) needs 3DS challenge iframe support

#### GAP-04: No Sandbox/Test Environment

**The Problem:** Merchants cannot test their integration before going live. Missing:

- ❌ Sandbox API key generation (separate from live)
- ❌ Test card numbers per scheme (Visa, Mastercard, Amex, Mada)
- ❌ Test acquirer responses (approve, decline, 3DS required, etc.)
- ❌ Test webhook events (simulate payment lifecycle)
- ❌ Test credential validation without real money movement
- ❌ Sandbox data isolation from production

#### GAP-05: No Circuit Breaker for Acquirer Connections

**The Problem:** The documentation mentions "circuit breaker" in runbook scenarios but there's no systematic circuit breaker pattern. If an acquirer's API starts failing (5xx errors, timeouts), ALL payment attempts to that acquirer should be automatically rerouted, not just failing per-intent.

```rust
// Proposed: circuit breaker for each MerchantAcquirerLink
pub struct CircuitBreakerState {
    pub link_id: Uuid,
    pub state: BreakerState,         // Closed | Open | HalfOpen
    pub failure_count: u32,
    pub last_failure_at: Option<DateTimeWithTimeZone>,
    pub opened_at: Option<DateTimeWithTimeZone>,
    pub half_open_attempts: u32,
}

pub enum BreakerState {
    Closed,     // Normal operation
    Open,       // Failing — skip this link
    HalfOpen,   // Testing if recovery happened
}
```

### 2.2 HIGH Priority Gaps

#### GAP-06: No API Versioning Strategy for Merchant-Facing API

The docs mention `proto-002` for service versioning but no merchant-facing API versioning. Merchants need:
- Stable API versions (v1, v2) with deprecation windows
- Changelog for breaking changes
- Migration guides between versions

#### GAP-07: No Bulk Operations API

High-volume merchants need:
- Bulk refunds (refund 100 payments at once)
- Bulk captures (capture 100 authorized payments)
- Bulk status checks
- Batch settlement reconciliation export
- Async job status tracking for bulk operations

#### GAP-08: No Merchant Analytics Dashboard Specs

The analytics-service has read endpoints but:
- ❌ No merchant-facing dashboard analytics (transaction volume, success rate, fee breakdown, chargeback rate)
- ❌ No exportable reports (CSV, Excel, PDF)
- ❌ No scheduled report delivery (email daily/weekly/monthly reports)
- ❌ No reconciliation report for merchant accounting

#### GAP-09: No Network Token Management

Visa and Mastercard are phasing out PAN-based transactions in favor of network tokens (Visa Token Service, Mastercard MDES). The platform needs:
- Network token provisioning during card-on-file setup
- Token replacement when card is reissued
- Falling back to network token when PAN is not available
- Storing both network token and acquirer token

#### GAP-10: No Account Updater Integration

Visa Account Updater (VAU) and Mastercard Automatic Billing Updater (ABU) provide updated card details when a cardholder's card is reissued. This is CRITICAL for subscription/recurring payments to prevent involuntary churn.

### 2.3 MEDIUM Priority Gaps

#### GAP-11: No Idempotency on All Mutating Commands

Only `CreatePaymentIntent` has caller-facing idempotency. All mutating commands should support idempotency keys:
- CapturePaymentIntent
- VoidPaymentIntent  
- RefundPaymentIntent
- CreateInvoice
- CreateSubscription
- ActivateRoutingPolicy

#### GAP-12: No Multi-Tenant Data Isolation Strategy

The platform is described as "single-tenant" but this is unclear. Does each operator get:
- Separate database schema? Separate database instance? Separate cluster?
- How is data isolation enforced at the infrastructure level?

#### GAP-13: No Statement Descriptor Management

Merchants need to control what appears on their customers' credit card statements:
- Static descriptor per merchant
- Dynamic descriptor per transaction (order ID, invoice number)
- Soft descriptor (what appears on statement)
- Hard descriptor (what's registered with the card scheme)

#### GAP-14: No Refund Reason Tracking

Refunds need reasons for analytics and compliance:
- Duplicate transaction
- Customer request
- Fraudulent transaction
- Goods/services not provided
- Other (with free-text)

#### GAP-15: No Dispute Prevention

Before a chargeback happens, the platform should:
- Send email receipts (via notification-service)
- Provide clear merchant descriptor
- Offer refund proactively for small disputes
- Track dispute win/loss rate for underwriting

---

## 3. Design Patterns Analysis

### 3.1 What's Done Well

#### ✅ Event Sourcing for Core Money-Movement Services

The choice to use event sourcing for orchestration, reconciliation, disputes, and subscriptions is **correct**:
- Full audit trail of money movement
- Ability to rebuild state from events
- Temporal queries ("what was the state at time X?")
- Naturally append-only (immutable audit)

#### ✅ Transactional Outbox Pattern

Writing events in the same Postgres transaction as the state change is the gold standard for reliable event publishing. Well documented.

#### ✅ Anti-Corruption Layer for Acquirers

`connector-gateway` as an ACL is the right pattern. Normalizing N different acquirer APIs into a single internal protocol is essential.

#### ✅ Protobuf for Internal Service Contracts

Using protobuf for service-to-service communication provides:
- Type safety
- Schema evolution
- Code generation
- Performance

#### ✅ Clear Separation of Commands and Queries (CQRS)

Write models use aggregates and event sourcing; read models use ClickHouse projections. Clean separation.

### 3.2 Problematic Patterns

#### ⚠️ Mixed Event Sourcing — Inconsistent Application

| Service | Pattern | Should It Be Event-Sourced? |
|---|---|---|
| orchestration-service | Event Sourced | ✅ Yes — money movement |
| reconciliation-service | Event Sourced | ✅ Yes — financial matching |
| dispute-service | Event Sourced | ⚠️ Maybe — relatively simple CRUD workflow |
| subscription-service | Event Sourced | ✅ Yes — billing lifecycle |
| invoice-service | CRUD + events | ⚠️ Invoice state changes involve money → should be event-sourced for audit |
| payment-link-service | CRUD + events | ✅ OK — no money movement directly |
| compliance-service | CRUD + events | ✅ OK — process workflow |
| operator-service | CRUD + events | ✅ OK — simple lifecycle |

**Issue:** Why is `invoice-service` CRUD while dealing with payment amounts and status? Invoices are financial documents and should have full event sourcing for audit.

#### ⚠️ In-Process Connector Adapters — Deployment Anti-Pattern

> From docs: "adapters are compiled into connector-gateway as plugins/modules, not separate microservices per acquirer"

**Problem:** Adding a new acquirer requires recompiling and redeploying `connector-gateway`. This couples connector deployment to the gateway deployment, creates risk (a bug in one adapter affects all), and prevents independent scaling.

**Better approach:** Each adapter as a separate process (can be same codebase, different binary) communicating via gRPC. Or use a plugin architecture with dynamic loading (e.g., WASM plugins).

#### ⚠️ Saga Coordinator as a Separate Service

A separate `saga-coordinator` service is over-engineering for the described sagas. The Payment Lifecycle Saga (Authorize → Capture → Settle → Reconcile) is already handled by `orchestration-service`'s state machine. Only cross-service sagas (like subscription renewal → create payment → handle dunning) need coordination.

**Better approach:** Inline saga coordination within the initiating service, or a lightweight saga library rather than a full service.

#### ⚠️ Protobuf-Only External API — Merchant Adoption Friction

The documentation is adamant: **no REST, no GET, no path variables, no query strings.**

This is a significant adoption barrier. Every major payment gateway uses RESTful JSON:
- **Stripe:** RESTful JSON API (de facto industry standard)
- **Adyen:** JSON over HTTP
- **Checkout.com:** RESTful JSON
- **PayPal:** RESTful JSON

**Why merchants want REST JSON:**
1. No protobuf compilation step
2. Works with any HTTP client (curl, Postman, any language)
3. Easy to debug (human-readable)
4. No schema generation for dynamic languages (Python, Ruby, PHP)
5. Familiar to ALL developers, not just those using protobuf

**Recommendation:** Support BOTH:
- Primary: RESTful JSON API (for merchant adoption)
- Secondary: Protobuf-over-HTTP (for performance-sensitive use cases)
- Internal: gRPC (service-to-service)

---

## 4. Over-Engineering Analysis

### 4.1 CRITICAL Over-Engineering — Will Slow You Down

#### 🚨 18 Microservices for Launch: Architecture Splinter

The platform proposes 18 microservices plus 2 gateways plus saga coordinator plus webhook delivery = **21 deployable services**.

**Reality check:**
- Stripe started with a monolith
- Adyen started with a monolith
- PayPal started with a monolith
- Square started with a monolith

Each service means:
- Its own CI/CD pipeline
- Its own monitoring dashboard
- Its own database connection pool
- Its own deployment (with health checks, readiness probes, rolling updates)
- Its own logging configuration
- Its own observability (metrics, traces, logs)
- Potential for N-service incident investigations

**Comparison:** A typical medium-scale payment platform runs 5-8 services at launch.

**Recommended consolidation:**

| Current | Proposed Merge | Rationale |
|---|---|---|
| operator-service | **Keep separate** | Foundation service |
| iam-service | **Keep separate** | Security boundary |
| compliance-service | **Merge with operator** | Both are onboarding |
| connector-gateway | **Keep separate** | Critical ACL |
| orchestration-service | **Keep separate** | Core engine |
| invoice-service | **Merge with orchestration** | Both deal with payment lifecycle |
| payment-link-service | **Keep separate** | Different scaling profile |
| subscription-service | **Keep separate** | Different scheduling profile |
| reconciliation-service | **Keep separate** | Financial matching |
| dispute-service | **Keep separate** | Different workflow |
| risk-service | **Merge with orchestration** | Called synchronously in hot path |
| ai-assistant-service | **Keep separate** | Different infrastructure (GPU) |
| ai-gateway | **Merge with api-gateway** | Not enough complexity to warrant separate service |
| document-service | **Merge with compliance** | Only compliance needs document upload at launch |
| notification-service | **Keep separate** | Different delivery characteristics |
| analytics-service | **Library/process** | Can be a read model built from events, not a separate service initially |
| saga-coordinator | **Library** | Not complex enough for a dedicated service |
| api-gateway | **Keep separate** | Single ingress |
| webhook-delivery | **Merge with notification** | Both are outbound delivery |

**Revised count: 10-12 services instead of 18.**

#### 🚨 ClickHouse at Launch

ClickHouse is excellent for analytics at scale, but at launch:
- Add another stateful service to manage
- Adds operational complexity (schema migrations, data retention, backups)
- PostgreSQL can handle analytics at small scale

**Recommendation:** Use PostgreSQL tables for analytics initially. Migrate to ClickHouse when event volume exceeds PostgreSQL's ability to aggregate efficiently (usually > 10M events/month).

#### 🚨 OpenSearch for Vector Search Before Scale

OpenSearch is:
- Another stateful service to manage
- Memory-intensive (JVM)
- Requires cluster management

For MVP, `pgvector` extension on PostgreSQL would handle the AI assistant's semantic search needs. Migrate to OpenSearch if/when vector search becomes a bottleneck.

#### 🚨 MinIO for Document Storage

MinIO is:
- Another stateful service
- Requires persistent volumes
- Needs backup configuration

For MVP, use the cloud provider's S3-compatible storage directly (AWS S3, GCS, Azure Blob). MinIO adds value only when running on-premise or in air-gapped environments.

#### 🚨 Tamper-Evident Audit Chain (Hash Linking)

Building a Merkle-tree-style audit chain is over-engineering:
- Adds complexity to every audit write
- WAL (Write-Ahead Log) in PostgreSQL already provides append-only guarantee
- Database replication provides redundancy
- Cloud provider backups provide recovery

**Better approach:** Append-only audit tables + database-level access controls + regular log shipping to immutable storage (S3 with object lock).

### 4.2 MEDIUM Over-Engineering

#### ⚠️ WebAuthn MFA for MVP

WebAuthn (hardware security keys) is excellent but:
- MVP should support TOTP (Google Authenticator, Authy)
- WebAuthn adds significant UX complexity
- Most operators don't have hardware security keys
- Can be added as an H1 upgrade

#### ⚠️ Seat-Based Concurrent Request Limiting

Counting in-flight requests per API key with Redis INCR/DECR adds:
- Risk of leaked counters (though TTL safety net helps)
- Additional Redis operations per request
- Complexity in error paths (when to DECR?)

**Simpler approach:** Fixed per-key rate limiting (Redis sliding window) is sufficient for MVP.

#### ⚠️ Distributed Tracing with W3C Trace Context Through NATS

Adding distributed trace context through async NATS events is complex:
- NATS events are asynchronous — spans can't be children
- Linked spans require careful correlation
- Most tracing tools handle synchronous spans better

**Simpler approach:** Log correlation via correlation_id is sufficient for debugging most issues. Full distributed tracing can be added post-launch.

#### ⚠️ Event Schema Registry as Git Repository

A git-based schema registry requires:
- PR workflow for schema changes
- CI validation pipeline
- Manual version bumps

**Simpler approach:** Shared protobuf module in a workspace crate with CI checks. Git registry becomes important only at > 5 teams or > 10 services.

---

## 5. Payment Router Perspective

### 5.1 What Modern Payment Routers Do

As someone who's built payment routing systems, here's what the orchestration engine needs:

#### 5.1.1 Multi-Dimensional Route Optimization

Current design: Priority-based routing with failover.

**What's needed for production:**
- **Cost-based routing:** Route to cheapest acquirer that can authorize the transaction
- **Success-rate routing:** Prefer acquirers with highest authorization rate for this card/currency/amount
- **Latency routing:** Route based on historical response time
- **Hybrid:** Weighted combination of cost, success rate, and latency
- **BIN-specific routing:** Specific card ranges (BINs) must go to specific acquirers
- **Country-specific routing:** Route domestic transactions to local acquirers (better rates, higher success)

#### 5.1.2 Intelligent Failover (Not Just Priority)

Current: Failover = try next priority.

**What's needed:**
- **Smart retry:** Don't retry the same card on the same acquirer (already done ✓)
- **BIN-aware failover:** If Visa declined on Acquirer A, try Visa on Acquirer B (not Mastercard on Acquirer A)
- **Amount-aware failover:** If high amount declined, try low-amount-tolerant acquirer
- **Merchant-configured fallback:** "If primary fails, use this specific backup"
- **Timeout-aware routing:** If acquirer is slow (> 3s), try next while waiting for first (optimistic racing)

#### 5.1.3 A/B Testing / Canary Routing

Merchants need to test new acquirers without risk:

```rust
pub struct CanaryConfig {
    pub acquirer_link_id: Uuid,
    pub traffic_percentage: f64,       // 0.01 = 1% of traffic
    pub conditions: Vec<RoutingCondition>, // only route matching transactions
    pub max_amount: Option<Money>,
    pub duration_days: u32,
    pub min_sample_size: u32,          // minimum transactions for statistical significance
    pub auto_promote: bool,            // automatically increase percentage if success rate > threshold
}
```

#### 5.1.4 Decline Code Intelligence

Not all declines are equal:

| Decline Type | Action |
|---|---|
| Insufficient Funds | Retry on different acquirer ✓ |
| Do Not Honor | May be transient — retry on different acquirer |
| Pick Up Card | FRAUD — do NOT retry |
| Lost/Stolen Card | BLOCK immediately, trigger security alert |
| Invalid CVV | Retry with correct CVV (merchant-side fix) |
| 3DS Required | Redirect to 3DS flow ❌ MISSING |
| Rate Limited | Back off and retry |
| Issuer Unavailable | Retry immediately on different acquirer |
| Transaction Not Allowed | May be card type restriction — retry on different acquirer |

**Current implementation** has a `is_retryable()` method but doesn't differentiate between:
- Retry immediately on different acquirer
- Retry after delay
- Do not retry (block)
- Retry with different parameter (e.g., with 3DS)

### 5.2 The Missing Payment Flow Scenarios

#### 5.2.1 Merchant-Initiated Transactions (MIT)

For subscriptions and recurring billing, the flow differs from Customer-Initiated Transactions (CIT):
- No 3DS required (exempt under PSD2)
- Must use stored credentials (network token or card-on-file token)
- Network mandate requires clear MIT indicator in authorization request
- Need to track: first MIT date, last MIT date, MIT type (unscheduled, recurring, installment)

#### 5.2.2 Unscheduled Credential-on-File (UCOF)

"Card on file" use cases not related to subscriptions:
- One-click checkout (Amazon-style)
- Installment payments
- Delayed capture (hotel/car rental)

Each has specific network mandate requirements (Visa, Mastercard, Amex all have different rules).

#### 5.2.3 Incremental Authorizations

For scenarios where the final amount is unknown at checkout:
- Hotels (estimate at check-in, finalize at check-out)
- Rental cars (estimate at pickup, finalize at return)
- Tipping (restaurant pre-auth + tip)

An incremental authorization increases the existing authorization amount without capturing.

#### 5.2.4 Delayed Settlement / Shipment-Based Capture

For physical goods merchants:
- Authorize at checkout (hold funds)
- Capture only when shipped (may be days later)
- Partial shipment → partial capture
- Cancelled item → partial void

The current flow supports this but could be better documented as a key merchant use case.

---

## 6. BYOK Model Deep Dive

### 6.1 What "BYOK" Means for Payment Orchestration

The user emphasized: **"one integration and BYOK to start using respective Payment Gateway — not like payment gateway."**

This means:
- **Payment Orchestra is NOT a payment gateway** — it doesn't process payments directly
- **Payment Orchestra is NOT a payment facilitator** — it doesn't aggregate merchants
- **Payment Orchestra IS a routing layer** that sits between merchants and their chosen payment gateways
- **Merchants bring their OWN merchant accounts with their OWN gateways**

### 6.2 BYOK Flow

```
1. Merchant signs up → creates operator account
2. Merchant completes KYB → operator becomes active
3. Merchant goes to "Gateway Configuration" page
4. Merchant selects gateway: "Network International", "Checkout.com" or "Telr"
5. Merchant enters their OWN credentials:
   - API Key, API Secret, Merchant ID from Network International
   - Secret Key, Public Key from Checkout.com
   - Merchant Hash, Authentication Key from Telr
6. Platform VALIDATES credentials (test API call)
7. On success → MerchantAcquirerLink created with status = Active
8. Merchant configures routing:
   - Priority 1: Network International (highest success rate)
   - Priority 2: Checkout.com (lower fees)
   - Priority 3: Telr (fallback)
9. Merchant configures conditions:
   - Route Visa/Mastercard to NI
   - Route Amex to Checkout.com
   - Route amounts > 50,000 AED to Telr
10. Merchant goes live — traffic flows through configured routing
```

### 6.3 BYOK-Specific Requirements

| Requirement | Current Status |
|---|---|
| **BYOK-01**: Merchant provides their own gateway credentials | ❌ Missing — no MerchantAcquirerLink service |
| **BYOK-02**: Credential validation before saving | ❌ Missing |
| **BYOK-03**: Credential rotation without service interruption | ❌ Missing |
| **BYOK-04**: Multiple credential sets per gateway (for rotation) | ❌ Missing |
| **BYOK-05**: Per-merchant routing configuration | ⚠️ Partially present in RoutingPolicy |
| **BYOK-06**: Per-merchant gateway profile limits | ⚠️ Partially present in GatewayProfile |
| **BYOK-07**: Gateway health monitoring per merchant | ❌ Missing |
| **BYOK-08**: Sandbox credentials separate from production | ❌ Missing |
| **BYOK-09**: Gateway-specific onboarding schema | ⚠️ Present in connector onboarding_schema() |
| **BYOK-10**: Credential expiry notification | ❌ Missing |
| **BYOK-11**: Gateway-specific test card numbers | ❌ Missing |
| **BYOK-12**: Merchant can see fee breakdown per gateway | ❌ Missing |
| **BYOK-13**: Merchant can see settlement reports per gateway | ❌ Missing — no merchant-facing reports |

### 6.4 BYOK Credential Management Architecture

```rust
// Proposed credential storage
pub struct EncryptedCredentials {
    pub key_identifier: String,     // KMS key used for encryption
    pub encrypted_payload: Vec<u8>, // JSON with credential fields, envelope-encrypted
    pub credential_hash: String,    // SHA-256 of raw credentials for dedup/change detection
    pub rotated_at: Option<DateTimeWithTimeZone>,
    pub expires_at: Option<DateTimeWithTimeZone>,
}

// Per-connector credential schema
pub trait ConnectorCredentialSchema {
    fn required_fields(&self) -> Vec<CredentialField>;
    fn optional_fields(&self) -> Vec<CredentialField>;
    fn validate_credentials(&self, credentials: &EncryptedCredentials) -> Result<(), ValidationError>;
    fn test_connection(&self, credentials: &EncryptedCredentials) -> Result<ConnectionTestResult, ConnectorError>;
}

pub struct CredentialField {
    pub name: String,                   // "api_key"
    pub display_name: String,           // "API Key"
    pub field_type: CredentialFieldType, // Text | Password | File | Certificate
    pub validation: FieldValidation,    // Regex, min_length, max_length
    pub required: bool,
    pub help_text: String,              // Where to find this in the gateway's dashboard
    pub documentation_url: Option<String>,
}

pub struct ConnectionTestResult {
    pub success: bool,
    pub latency_ms: u32,
    pub merchant_name: Option<String>,   // Verified merchant name from gateway
    pub permissions: Vec<String>,        // What this credential can do (authorize, capture, refund, etc.)
    pub error_message: Option<String>,
}
```

### 6.5 BYOK Onboarding Flow (New Service: `merchant-acquirer-link-service`)

```
┌─────────────────────────────────────────────────────────────────┐
│                   BYOK Onboarding Flow                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Merchant → Dashboard → "Connect Gateway" → Select Connector    │
│       │                                                         │
│       ▼                                                         │
│  Fill Credential Form (dynamic per connector schema)             │
│       │                                                         │
│       ▼                                                         │
│  "Test Connection" → connector-gateway validates credentials     │
│       │                                                         │
│       ├── Success → Save encrypted credentials                  │
│       │              Create MerchantAcquirerLink (Active)        │
│       │              Create default GatewayProfile (from template)│
│       │              Start health monitoring                     │
│       │                                                         │
│       └── Failure → Show error message with help text            │
│                      "Invalid API Key. Check your NI dashboard." │
│                                                                  │
│  Merchant → Configure Routing Rules                              │
│  Merchant → Configure Gateway Profile (limits, fees)             │
│  Merchant → Go Live                                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. Service-by-Service Assessment

### 7.1 operator-service (BC-01)
**Assessment:** Good foundation. Simple, CRUD, well-defined.
**Issues:**
- Status field should be an enum, not String
- No subdomain provisioning logic detailed
- Missing: email verification rate limiting, duplicate detection

### 7.2 iam-service (BC-02)
**Assessment:** Comprehensive with good security controls.
**Issues:**
- ABAC is complex to implement correctly — expect this to be a major engineering effort
- Permission evaluation on critical path (every API call) — needs careful optimization
- Missing: API key last-used tracking, unused key auto-disabling, key usage analytics
- Maker/Checker as a separate aggregate adds complexity — could be a simpler workflow table

### 7.3 compliance-service (BC-03)
**Assessment:** Good KYB workflow with AML monitoring.
**Issues:**
- AML rules are hardcoded — need to be configurable per operator
- SAR generation is manual — should be semi-automated (draft for review)
- Missing: ongoing monitoring (not just at onboarding), sanctions list screening (OFAC, UN, EU)

### 7.4 connector-gateway (BC-04)
**Assessment:** Well-designed ACL. The core abstraction (`AcquirerConnector` trait) is solid.
**Issues:**
- **CRITICAL:** No circuit breaker per acquirer
- **CRITICAL:** In-process adapters = deployment coupling (see §3.2)
- Missing: 3DS support in authorize flow
- Missing: Network token provisioning
- Missing: Credential rotation support (swap credentials without interrupting active transactions)
- Missing: Rate limiting per connector (mentioned but not implemented)
- Fee calculation with tiered pricing is documented but marked as gap

### 7.5 orchestration-service (BC-05)
**Assessment:** The heart of the platform. Well-designed state machine and routing.
**Issues:**
- **CRITICAL:** No 3DS support
- **CRITICAL:** No circuit breaker integration
- Missing: Smart/weighted routing beyond priority
- Missing: A/B testing / canary routing
- Missing: Incremental authorization
- Missing: Merchant-specific routing overrides
- Missing: Latency budget enforcement per hop
- State machine diagram is good but doesn't include `RefundPending` state (REFUND-EDGE-002)

### 7.6 invoice-service (BC-06)
**Assessment:** Basic CRUD, which is insufficient for financial documents.
**Issues:**
- Should be event-sourced for audit trail
- Missing: invoice templates, PDF generation, tax calculation (VAT for UAE)
- Missing: invoice reminders, overdue escalation workflow
- Missing: credit notes, refund invoices, proforma invoices

### 7.7 payment-link-service (BC-07)
**Assessment:** Good concept, well-separated from invoice-service.
**Issues:**
- Missing: payment link expiry notification, QR code generation
- Missing: embedded checkout vs hosted page (merchant embedding)
- Missing: localization/localization (UAE supports Arabic + English)
- Missing: Apple Pay / Google Pay / Mada button integration

### 7.8 subscription-service (BC-08)
**Assessment:** Comprehensive billing lifecycle.
**Issues:**
- Dunning retry logic needs more detail (retry schedule, max attempts, final action)
- Missing: subscription proration (mid-cycle plan changes) — acknowledged as gap
- Missing: subscription metrics (MRR, ARR, churn rate, LTV)
- Missing: usage-based billing (not just flat/recurring)
- Missing: free trial → paid conversion logic
- Missing: pause/resume with proration (acknowledged)

### 7.9 reconciliation-service (BC-09)
**Assessment:** Solid reconciliation matching. The gap analysis is honest and identifies several needs.
**Issues:**
- T+N settlement tracking is marked as gap — needs to be built before launch
- Fee variance tracking is marked as gap — needs to be built before launch
- The matching algorithm is well-defined with confidence levels
- Settlement adjustments handling is marked as gap

### 7.10 dispute-service (BC-10)
**Assessment:** Functional chargeback management.
**Issues:**
- Representment deadline tracking is marked as gap — CRITICAL for merchant success (missing deadline = automatic loss)
- Chargeback notification flow is well-designed
- Missing: chargeback analytics (win/loss rate by reason code, by acquirer)
- Missing: automated evidence collection (transaction receipt, communication logs)
- Missing: pre-dispute resolution (refund before chargeback)

### 7.11 risk-service (BC-11)
**Assessment:** MVP rule-based scoring is adequate.
**Issues:**
- Missing: integration with orchestration-service routing (acknowledged)
- Risk-based routing rule (route high-risk to specific acquirer) is marked as gap
- Missing: negative database (known fraud indicators)
- Missing: 3DS exemption recommendation (low-risk → no challenge)
- Velocity checks are good but need per-operator configuration

### 7.12 ai-assistant-service (BC-12)
**Assessment:** Differentiated feature — AI-powered payment assistant.
**Issues:**
- Missing: how the AI handles temporal queries ("what was the success rate last Tuesday?")
- Missing: AI action execution (user says "refund payment X" → AI creates refund)
- Privacy concerns with AI accessing transaction data (documented but needs more detail)
- Model accuracy monitoring and fallback
- Missing: conversation history/context management

### 7.13 document-service (BC-13)
**Assessment:** MinIO-backed blob storage with OCR.
**Issues:**
- Over-engineered for MVP — compliance documents only
- Could be merged into compliance-service initially
- OCR via Qwen3-VL 8B is ambitious — will this be fast enough for synchronous KYB flow?

### 7.14 notification-service (BC-14)
**Assessment:** Standard notification dispatch.
**Issues:**
- Missing: notification templates (configurable per operator)
- Missing: notification preferences per operator (email, SMS, push)
- Missing: delivery analytics (open rates, click rates for email)
- Missing: webhook delivery should be merged here (both are outbound delivery)

### 7.15 analytics-service (BC-15)
**Assessment:** ClickHouse-powered event analytics.
**Issues:**
- ClickHouse at launch is premature (see §4)
- Missing: merchant-facing dashboard endpoints
- Missing: report generation (PDF, CSV export)
- Missing: scheduled report delivery
- Missing: anomaly detection (unusual drop in authorization rate)

### 7.16 saga-coordinator (BC-17)
**Assessment:** Over-engineered as separate service.
**Issues:**
- Saga orchestration should be a library, not a service
- The "Payment Lifecycle" saga duplicates orchestration-service's state machine
- Missing: saga timeout detection via background sweep (documented)
- Compensation retry logic is well-designed

### 7.17 api-gateway
**Assessment:** Well-defined single ingress.
**Issues:**
- Protobuf-only is a poor choice for merchant API (see §3.2)
- Missing: request validation schema (beyond protobuf decode)
- Missing: API key scoping (per-resource, per-operation)
- CORs policy is good

### 7.18 ai-gateway
**Assessment:** Premature separation from api-gateway.
**Issues:**
- The guardrail, quota, and routing functionality could be middlewares in api-gateway
- Adding another service for AI traffic control adds operational complexity
- Circuit-break to degraded mode is good, but could be handled without a separate service

### 7.19 Infrastructure & Cross-Cutting
**Assessment:** Well-thought-out infrastructure patterns.
**Issues:**
- Over-engineered: hash-linked audit chain, concurrent request counting, distributed tracing through NATS
- Missing: detailed backup/restore procedures for each stateful service
- Missing: disaster recovery plan across regions
- Missing: cost allocation (track infrastructure cost per operator)
- Missing: capacity planning guidelines (when to scale what)

---

## 8. Critical Recommendations

### 8.1 MUST DO Before Launch

| Priority | Action | Area |
|---|---|---|
| 🔴 P0 | Build `MerchantAcquirerLink` service with full CRUD + credential validation | BYOK |
| 🔴 P0 | Build webhook delivery service (merge with notification-service) | Merchant Integration |
| 🔴 P0 | Add 3D Secure support to authorization flow | Compliance |
| 🔴 P0 | Build sandbox/test environment with test card numbers | Merchant Onboarding |
| 🔴 P0 | Implement circuit breaker per acquirer connection | Reliability |
| 🔴 P0 | Add merchant-facing dashboard (transaction list, search, filter) | Merchant Experience |

### 8.2 SHOULD DO Before Launch

| Priority | Action | Area |
|---|---|---|
| 🟠 P1 | Reduce service count from 18 to 10-12 | Architecture |
| 🟠 P1 | Support RESTful JSON API alongside protobuf for merchants | Developer Experience |
| 🟠 P1 | Add idempotency to all mutating commands | Reliability |
| 🟠 P1 | Build merchant SDK/client library (at least one: Node.js, Python, or curl) | Developer Experience |
| 🟠 P1 | Implement T+N settlement tracking (SettlementExpectation) | Financial |
| 🟠 P1 | Implement fee variance tracking | Financial |
| 🟠 P1 | Implement representment deadline tracking | Disputes |
| 🟠 P1 | Add API versioning strategy and migration guide | API Design |

### 8.3 COULD DO Post-Launch

| Priority | Action | Area |
|---|---|---|
| 🟡 P2 | Weighted/ML-based routing (success rate, cost, latency) | Optimization |
| 🟡 P2 | A/B testing / canary routing framework | Optimization |
| 🟡 P2 | Network token management | Compliance |
| 🟡 P2 | Account updater integration (VAU/ABU) | Subscriptions |
| 🟡 P2 | Bulk operations API | Scale |
| 🟡 P2 | Scheduled report delivery (email daily/weekly reports) | Merchant Experience |
| 🟡 P2 | Subscription proration engine | Billing |

### 8.4 DON'T DO (Over-Engineering)

| ❌ Avoid | Alternative |
|---|---|
| Separate saga-coordinator service | Saga library within orchestration-service |
| Separate ai-gateway service | Middleware in api-gateway |
| ClickHouse at launch | PostgreSQL analytics tables |
| OpenSearch at launch | pgvector for semantic search |
| MinIO at launch | Direct S3 API usage |
| Hash-linked audit chain | Append-only audit tables |
| WebAuthn MFA for MVP | TOTP (Google Authenticator) |
| Concurrent request counting | Simple rate limiting |
| Event schema registry as Git repo | Shared protobuf workspace crate |

---

## 9. Revised Architecture Proposal

### 9.1 Service Map (Revised)

```
                                    ┌─────────────────────┐
                                    │    Merchant Client   │
                                    │  (REST JSON / SDK)  │
                                    └──────────┬──────────┘
                                               │
                                    ┌──────────▼──────────┐
                                    │     api-gateway      │
                                    │  (Axum + middleware) │
                                    │  auth, rate-limit,   │
                                    │  cors, logging       │
                                    └───┬──────┬──────┬───┘
                                        │      │      │
                    ┌───────────────────┘      │      └───────────────────┐
                    ▼                          ▼                          ▼
          ┌─────────────────┐       ┌──────────────────┐      ┌──────────────────┐
          │   iam-service   │       │  orchestration-   │      │  connector-       │
          │   (auth, ABAC)  │       │  service (core)   │      │  gateway (ACL)    │
          │   JWT, API Keys│       │  + saga lib       │      │  + circuit        │
          │   MFA, Sessions│       │  + risk scoring   │      │  breaker          │
          └─────────────────┘       │  + invoice logic  │      └───────┬──────────┘
                                    │  + 3DS handling   │              │
                                    └────────┬─────────┘              │
                                             │                         │
                                    ┌────────▼─────────┐     ┌────────▼──────────┐
                                    │  NATS JetStream   │     │  Acquirer A       │
                                    │  (event bus)      │     │  Acquirer B       │
                                    └────────┬─────────┘     │  Acquirer C       │
                                             │               └───────────────────┘
                    ┌───────────────────────┬┼┬───────────────────────┐
                    ▼                       ▼▼▼                       ▼
          ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
          │  subscription-   │  │  reconciliation-  │  │  notification-    │
          │  service         │  │  service          │  │  service          │
          │  (billing)       │  │  (settlements)    │  │  (email/SMS +     │
          └──────────────────┘  └──────────────────┘  │   webhook)        │
                                                       └──────────────────┘
          ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
          │  dispute-service  │  │  ai-assistant-   │  │  compliance-      │
          │  (chargebacks)    │  │  service (RAG)   │  │  service          │
          └──────────────────┘  └──────────────────┘  │  (KYB + AML)      │
                                                       └──────────────────┘
          ┌──────────────────┐  ┌──────────────────┐
          │  payment-link-   │  │  analytics-       │
          │  service (hosted)│  │  service (read)   │
          └──────────────────┘  └──────────────────┘
```

**Revised service count: 12 (from 21)**

### 9.2 Merge Rationale

| Merged From → Into | Reasoning |
|---|---|
| ai-gateway → api-gateway | AI guardrails are middleware; not a full service |
| saga-coordinator → orchestration-service | Saga orchestration is a library, not a service |
| risk-service → orchestration-service | Called synchronously in hot path; same deploy unit |
| invoice-service → orchestration-service | Invoice is just another view of PaymentIntent data |
| webhook-delivery → notification-service | Both are outbound delivery mechanisms |
| document-service → compliance-service | Only compliance needs document upload at MVP |
| analytics-service → standalone service | Keep separate but no ClickHouse at launch |

### 9.3 Database Strategy (Simplified)

| Service | Primary DB | Cache | Notes |
|---|---|---|---|
| iam-service | PostgreSQL | Redis | Token blacklist, permission cache |
| orchestration-service | PostgreSQL | Redis | Event store + idempotency cache |
| connector-gateway | PostgreSQL (config only) | — | Minimal state |
| subscription-service | PostgreSQL (event store) | Redis | Billing schedule cache |
| reconciliation-service | PostgreSQL (event store) | — | |
| dispute-service | PostgreSQL | — | CRUD + events, not full event sourcing |
| ai-assistant-service | PostgreSQL | OpenSearch (H1) | Start with pgvector |
| notification-service | PostgreSQL | Redis | Delivery dedup |
| compliance-service | PostgreSQL | — | |
| payment-link-service | PostgreSQL | Redis | Short-lived link cache |
| analytics-service | PostgreSQL → ClickHouse (H2) | — | Start Postgres, migrate when needed |

---

## Appendix A: Connector-Gateway Trait (BYOK-Enhanced)

```rust
#[async_trait]
pub trait AcquirerConnector: Send + Sync {
    fn connector_id(&self) -> ConnectorId;
    fn capabilities(&self) -> ConnectorCapabilities;
    fn onboarding_schema(&self) -> OnboardingSchema; // Credential fields & validation

    // Core payment operations
    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError>;
    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError>;
    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError>;
    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError>;

    // NEW: 3DS support
    async fn check_3ds_enrollment(&self, req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError>;
    async fn authenticate_3ds(&self, req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError>;

    // NEW: Network token support
    async fn provision_network_token(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError>;

    // NEW: Transaction status check
    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError>;

    // Settlement
    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError>;
    fn settlement_cycle(&self) -> SettlementCycle;

    // NEW: Credential management
    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError>;
    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError>;

    // Webhooks
    fn verify_webhook_signature(&self, headers: &HeaderMap, body: &[u8]) -> Result<(), ConnectorError>;
    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError>;
}
```

## Appendix B: New Endpoints for Merchant-Facing API

```protobuf
// Merchant-Facing RESTful JSON Endpoints (not protobuf)
// All POST /v1/{resource} patterns

// Core Payment API
POST /v1/payment-intents               // CreatePaymentIntent
POST /v1/payment-intents/:id/authorize  // AuthorizePaymentIntent
POST /v1/payment-intents/:id/capture    // CapturePaymentIntent
POST /v1/payment-intents/:id/void       // VoidPaymentIntent
POST /v1/payment-intents/:id/refund     // RefundPaymentIntent
GET  /v1/payment-intents/:id            // GetPaymentIntent
GET  /v1/payment-intents                // ListPaymentIntents (with filters)

// Gateway Configuration (BYOK)
POST /v1/merchant-links                 // CreateMerchantAcquirerLink
GET  /v1/merchant-links                 // ListMerchantAcquirerLinks
GET  /v1/merchant-links/:id             // GetMerchantAcquirerLink
POST /v1/merchant-links/:id/test        // TestMerchantAcquirerConnection
POST /v1/merchant-links/:id/rotate      // RotateCredentials
PATCH /v1/merchant-links/:id            // UpdateMerchantAcquirerLink
DELETE /v1/merchant-links/:id           // DisableMerchantAcquirerLink

// Connector Catalog (discoverable)
GET  /v1/connectors                     // List available connectors
GET  /v1/connectors/:id/schema          // Get credential schema for connector
GET  /v1/connectors/:id/test-cards      // Get test card numbers for connector

// Routing Configuration
POST /v1/routing-policies               // CreateRoutingPolicy
GET  /v1/routing-policies               // ListRoutingPolicies
GET  /v1/routing-policies/:id           // GetRoutingPolicy
POST /v1/routing-policies/:id/activate  // ActivateRoutingPolicy

// Webhook Configuration (for outbound merchant webhooks)
POST /v1/webhook-endpoints              // CreateWebhookEndpoint
GET  /v1/webhook-endpoints              // ListWebhookEndpoints
POST /v1/webhook-endpoints/:id/test     // Send test webhook event
GET  /v1/webhook-endpoints/:id/deliveries // View delivery history
POST /v1/webhook-endpoints/:id/retry/:delivery_id // Retry failed delivery

// Reporting & Analytics
GET  /v1/analytics/transactions         // Transaction volume & success rates
GET  /v1/analytics/fees                 // Fee breakdown by gateway
GET  /v1/analytics/chargebacks          // Chargeback analytics
GET  /v1/analytics/settlements          // Settlement reports
GET  /v1/reports/daily                 // Daily reconciliation report
GET  /v1/reports/monthly               // Monthly statement
```
