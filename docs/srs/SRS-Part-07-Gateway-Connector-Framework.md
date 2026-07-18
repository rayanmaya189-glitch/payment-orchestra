# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 7 of 12:** Gateway Connector Framework
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 7 of 12 — Gateway Connector Framework |
| Depends On | Part 3 (BC-04 as Anti-Corruption Layer/Open Host Service), Part 4 (SVC-04 `connector-gateway`), Part 5 (calls this framework's normalized API on the checkout hot path) |
| Feeds Into | Part 9 (connector config schema, settlement staging tables), Part 10 (the normalized internal gRPC contract every adapter implements), Part 11 (per-connector conformance test suite as part of TDD strategy) |
| Purpose of This Part | Define the plugin architecture, normalized contract, capability model, and per-connector requirements that let `orchestration-service` and `reconciliation-service` remain acquirer-agnostic (BIZ-010). |

---

## 1. Architectural Shape

### 1.1 In-Process Plugin Model (Restated Rationale from Part 4 §4.3)

Each supported acquirer/PSP is implemented as a Rust module implementing a shared `AcquirerConnector` trait, compiled into the single `connector-gateway` service binary — not deployed as N separate microservices. This keeps operational overhead flat as the connector count grows and avoids network hops for what is fundamentally a protocol-translation concern. New connectors are added by implementing the trait and registering it in a connector registry, gated behind configuration (a connector can be present in the binary but disabled/unlicensed for a given deployment).

### 1.2 The Normalized Contract

```rust
#[async_trait]
pub trait AcquirerConnector: Send + Sync {
    fn connector_id(&self) -> ConnectorId;
    fn capabilities(&self) -> ConnectorCapabilities;

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError>;
    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError>;
    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError>;
    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError>;
    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError>;

    // Settlement ingestion — not every connector supports all mechanisms (§4)
    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError>;
    fn verify_webhook_signature(&self, headers: &HeaderMap, body: &[u8]) -> Result<(), ConnectorError>;
    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError>;

    // Onboarding/config validation
    fn onboarding_schema(&self) -> OnboardingSchema;
    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<(), ConnectorError>;
}
```

- **CONN-001**: Every method returns the platform's own normalized types (`AuthorizeResponse`, `DeclineReason`, etc. — Part 3 §3.1 value objects), never the acquirer's raw response shape. All acquirer-specific parsing/mapping happens inside the trait implementation, which is the ACL boundary in code form.
- **CONN-002**: `ConnectorError` is itself a normalized error taxonomy (e.g., `Declined(DeclineReason)`, `Timeout`, `AuthenticationFailed`, `RateLimited`, `UnsupportedOperation`) so that `orchestration-service`'s routing/retry logic (Part 5 §3.4) never has to special-case a specific acquirer's error format.

### 1.3 Capability Flags

Not every acquirer supports every operation identically. `ConnectorCapabilities` is a struct the routing/reconciliation logic consults before assuming a feature is available:

```rust
pub struct ConnectorCapabilities {
    pub supports_partial_capture: bool,
    pub supports_partial_refund: bool,
    pub supports_native_idempotency_key: bool,     // else engine uses status-check-before-retry (Part 5 §4.1)
    pub supports_webhook_settlement: bool,          // vs. polling-only or file-drop-only
    pub supports_realtime_status_check: bool,
    pub supports_marketplace_split: bool,           // relevant to BC-16 (Part 3/5 §6)
    pub supported_card_schemes: Vec<CardScheme>,
    pub supported_currencies: Vec<CurrencyCode>,
    pub settlement_format: SettlementFormat,        // Webhook | PollingApi | SftpFile | ScannedDocument
}
```

- **CONN-003**: `orchestration-service`'s routing algorithm (Part 5 §3.2) filters candidate acquirers not only by tenant-configured routing rules but also by whether the connector's capabilities satisfy the transaction's requirements (e.g., a partial-capture request is never routed to a connector with `supports_partial_capture = false`).

---

## 2. Onboarding Schema & Credential Handling

### 2.1 Dynamic Onboarding Schema

Each connector exposes an `OnboardingSchema` (a structured field list: field name, type, required/optional, validation regex) consumed by the merchant dashboard (UC-010, Part 2) to render the correct configuration form dynamically — the dashboard does not hard-code per-acquirer forms; it renders whatever schema the selected connector declares. This directly satisfies BIZ-010's "no code change" requirement extending into the connector-onboarding UI itself, not just routing rules.

### 2.2 Credential Security

- **CRED-001**: All connector credentials (API keys, merchant IDs treated as sensitive, webhook secrets) are encrypted at rest using envelope encryption (tenant-specific data key wrapped by a platform master key — full design in Part 8, Secrets Management).
- **CRED-002**: Credentials are never returned in plaintext via any read API after initial save — the dashboard shows a masked representation (e.g., last 4 characters) only (BR-010-2, Part 2).
- **CRED-003**: `validate_credentials` is called synchronously at connection time (UC-010 step 3) using a minimally-privileged sandbox/status-check call — never a live-money-movement call — to avoid accidentally charging anything during connector setup.

---

## 3. Decline Code Normalization

### 3.1 Why Normalization Matters

Each acquirer/scheme returns decline reasons in its own proprietary taxonomy (e.g., raw ISO 8583 response codes, or a PSP's own string enum). Part 5's routing/failover logic must reason about "is this retryable" and "why did this fail" in one consistent vocabulary — this is what makes BIZ-010 ("routing configurable without code change per acquirer") actually true in practice rather than aspirational.

### 3.2 Normalized `DeclineReason` Taxonomy (Representative Subset)

| Normalized Reason | Typically Retryable on a Different Acquirer? | Example Raw Sources Mapped |
|---|---|---|
| `InsufficientFunds` | Yes (different acquirer/issuer routing path can succeed) | ISO 8583 code 51, PSP-specific "NSF" strings |
| `DoNotHonor` | Sometimes (tenant-configurable default: yes) | ISO 8583 code 05 |
| `InvalidCard` | No (retrying elsewhere won't fix a genuinely invalid card) | ISO 8583 code 14 |
| `ExpiredCard` | No | ISO 8583 code 54 |
| `SuspectedFraud` | No (and should trigger risk-service flagging, Part 3 BC-11) | ISO 8583 code 59, PSP fraud-score rejections |
| `IssuerUnavailable` | Yes | ISO 8583 code 91, PSP "gateway timeout" |
| `ThreeDSecureFailed` | Depends (tenant-configurable) | PSP-specific 3DS failure codes |
| `RateLimitedByAcquirer` | Yes, on a different acquirer | HTTP 429 equivalents |
| `UnknownError` | Tenant-configurable default (conservative default: not retryable, to avoid masking real integration issues as routine declines) | Anything unmapped — logged for connector-mapping-table improvement |

- **DECL-001**: Every connector implementation maintains its own raw-code → normalized-reason mapping table, reviewed and updated as part of that connector's own test suite (Part 11) — an unmapped raw code must default to `UnknownError` and be logged/alerted (never silently mis-mapped to a plausible-sounding but wrong normalized reason).

---

## 4. Settlement Ingestion Per Connector

### 4.1 Supported Settlement Formats

| Format | Description | Handling |
|---|---|---|
| `Webhook` | Acquirer pushes settlement confirmation in near-real-time per transaction or small batch | `connector-gateway` exposes a dedicated, per-connector signed webhook endpoint (Part 4 §2.1 GW-005); `verify_webhook_signature` + `parse_webhook` normalize into `RawSettlementRecord` → published to NATS for `reconciliation-service` |
| `PollingApi` | Acquirer exposes a settlement-status API the platform must poll | JOB-003 (Part 4 §6) triggers `poll_settlement` on a schedule |
| `SftpFile` | Acquirer/bank delivers a batch file via SFTP | A dedicated file-watcher component (part of `connector-gateway`) picks up new files, validates checksum (Part 3 INV-06), and normalizes each line into `RawSettlementRecord` |
| `ScannedDocument` | Bank/acquirer only provides a PDF/scanned settlement advice (no structured feed) | Routed through `document-service` → Qwen3-VL 8B extraction (Part 6 §4) before normalization |

### 4.2 Normalization Target

Regardless of source format, every settlement input is normalized into the same `RawSettlementRecord` shape before being handed to `reconciliation-service` (Part 3 BC-09), so that `SettlementBatch`/`SettlementRecord` aggregate logic never needs to know which connector or format produced the data.

---

## 5. Circuit Breaker and Resilience Patterns

### 5.1 Per-Connector Circuit Breakers

- **CB-CONN-001**: Each acquirer adapter within `connector-gateway` maintains an independent circuit breaker (per Part 3 §9.3 CB-001). The circuit breaker state machine follows the standard Closed → Open → Half-Open pattern:
  - **Closed** (normal): All requests pass through. Error rate is tracked over a sliding window (default: 30 seconds).
  - **Open** (failing): When error rate exceeds the threshold (default: 50%), the circuit opens for a configurable duration (default: 60 seconds). All requests to this connector fail fast with `ConnectorError::CircuitOpen`, allowing `orchestration-service` to immediately route to the next candidate.
  - **Half-Open** (probing): After the open window, a single probe request is allowed through. If it succeeds, the circuit closes; if it fails, the circuit re-opens.

- **CB-CONN-002**: Circuit breaker state is cached in Redis for cross-replica visibility — if one `connector-gateway` replica detects a failing acquirer, all replicas skip that connector without independently discovering the failure.

- **CB-CONN-003**: The circuit breaker only tracks *acquirer-side* failures (timeouts, 5xx responses, authentication failures). Client-side errors (invalid requests, insufficient funds) do not trip the circuit — a high decline rate is a business condition, not a system health signal.

### 5.2 Bulkhead Isolation

- **BULK-CONN-001**: Each acquirer adapter has its own dedicated `reqwest::Client` connection pool with configurable pool limits and per-connection timeouts, isolated from other adapters. A slow/hanging connection to one acquirer cannot exhaust the connection pool shared with other adapters.
- **BULK-CONN-002**: Each adapter has a per-request timeout (default: 10 seconds for authorize/capture, 30 seconds for settlement polling) that is independent of other adapters' timeouts. The total checkout-path latency budget (Part 5 RTY-002) is enforced at the `orchestration-service` level, not within individual adapters.

### 5.3 Retry Configuration per Connector

Each connector adapter declares its retry behavior as part of `ConnectorCapabilities`:

```rust
pub struct ConnectorRetryConfig {
    pub max_retries: u8,                    // default: 1 (the initial attempt + 1 retry)
    pub initial_backoff_ms: u32,            // default: 100ms
    pub backoff_multiplier: f32,            // default: 2.0
    pub max_backoff_ms: u32,               // default: 5000ms
    pub jitter_percent: f32,               // default: 0.25 (±25%)
    pub retryable_error_codes: Vec<ConnectorError>, // which errors trigger retry
}
```

- **RETRY-CONN-001**: Connectors with native idempotency key support (`supports_native_idempotency_key = true`) can safely retry without a pre-flight status check. Connectors without native idempotency must perform a status check before retrying to avoid double-authorization (Part 5 §4.1).
- **RETRY-CONN-002**: The retry budget is shared with `orchestration-service`'s failover retry budget (RTY-002) — a retry within a single connector counts as one of the total allowed hops.

---

## 6. Connector Conformance Testing (Preview — Full Detail in Part 11)

- **CONF-001**: Every connector implementation must pass a shared **conformance test suite** exercising the `AcquirerConnector` trait against that connector's sandbox environment: successful authorize/capture/void/refund, each documented decline scenario, timeout handling, webhook signature verification (valid and tampered), and idempotency behavior (native or status-check-based per capability flags).
- **CONF-002**: This conformance suite is written *before* a new connector's production code (TDD discipline, Part 11) — a connector is not considered "done" until it passes 100% of the shared conformance suite plus any connector-specific edge cases documented in its own module.
- **CONF-003**: The conformance suite doubles as regression protection: any change to the shared `AcquirerConnector` trait or normalization logic must not break existing connectors' conformance results.

---

## 6. Initial MVP Connector Shortlist (Placeholder Pending OQ-003)

Per Part 1 OQ-003, the final MVP acquirer/PSP shortlist requires business confirmation. This SRS's connector framework is designed to be provider-agnostic, but for concreteness, the following UAE-relevant categories of provider are anticipated and should be validated against real API documentation once confirmed:

- A regional acquiring bank/processor (e.g., Network International or Magnati-class provider) — likely `Webhook` + `SftpFile` settlement mix.
- A regional PSP aggregator (e.g., Telr or PayTabs-class provider) — likely `PollingApi` or `Webhook` settlement.
- An international PSP with MENA presence (e.g., Checkout.com-class provider) — likely `Webhook` settlement, native idempotency key support.

**This list is illustrative only and must not be treated as a final vendor decision** — it exists so Part 9's settlement schema and Part 11's conformance test planning can proceed against a realistic shape while OQ-003 is resolved.

---

## 8. Traceability

| Requirement | Realized By |
|---|---|
| BIZ-010 (routing config without code change) | §1.2 normalized contract, §1.3 capability flags, §2.1 dynamic onboarding schema |
| BR-010-2 / CRED-002 (credentials never returned plaintext) | §2.2 |
| Part 5 §4.1 (idempotency layering) | §1.3 `supports_native_idempotency_key` capability flag |
| Part 5 §3.4 RTY-003 (normalized decline reasons for routing/audit) | §3 |
| Part 3 INV-06 (settlement idempotent ingestion) | §4.1 SftpFile checksum validation |
| Part 6 §4 (vision extraction for unstructured settlement) | §4.1 `ScannedDocument` row |
| Circuit Breaker / Bulkhead (connector resilience) | §5 CB-CONN-001 through BULK-CONN-002 |
| Retry configuration per connector | §5.3 RETRY-CONN-001, RETRY-CONN-002 |

---

## 9. Open Items Carried Forward

- **OQ-016 (= OQ-003 from Part 1, restated here for engineering visibility)**: Final MVP acquirer/PSP shortlist must be confirmed before connector implementation begins in earnest — §6's list is a planning placeholder only.
- **OQ-017**: Confirm whether webhook endpoints (§4.1) should be per-connector-per-tenant unique URLs (simplifies signature/source attribution) or a shared per-connector URL disambiguated by payload content — a Part 9/Part 10 API design decision affecting the webhook contract.
- **OQ-043**: Finalize circuit breaker thresholds (§5.1 CB-CONN-001) — error-rate percentage, sliding-window duration, and open-window duration — against real acquirer failure-mode data from pilot merchants.

---

*End of Part 7. Proceed to Part 8: Identity, Security & Compliance.*
