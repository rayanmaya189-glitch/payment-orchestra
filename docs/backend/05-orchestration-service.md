# 05 — orchestration-service (BC-05 Payment Orchestration)

The core domain service. Event-sourced. Owns `PaymentIntent` and `RoutingPolicy` aggregates.

---

## 1. Domain Model

### Aggregates

#### AGG-01: PaymentIntent (Aggregate Root)

**Identity**: `payment_intent_id: Uuid` (UUIDv7)

**Entities**:
- `RoutingAttempt` — one per acquirer hop attempted

**Value Objects**:
- `Money` (amount_minor_units: i64, currency: CurrencyCode)
- `IdempotencyKey` (String, caller-supplied)
- `AcquirerReference` (String, opaque)
- `DeclineReason` (normalized enum)
- `FeeBreakdown` (interchange, scheme, acquirer markup, processing, total)

**State Machine**:
```
Created → Authorizing → Authorized → Capturing → Captured
                                            ↓
                                    PartiallyCaptured → (more captures) → Captured
                                            ↓
                                       Refunding → Refunded / PartiallyRefunded

Created → Authorizing → Failed (single-hop terminal)
Created → Authorizing → FailedAllRoutes (terminal)

Authorized → Voided
Authorized → AuthorizationExpired (time-based, JOB-007)
```

**Invalid State Transitions** (must be rejected with deterministic error):

| Current State | Command | Error Code |
|---|---|---|
| `Failed` / `FailedAllRoutes` | `CapturePaymentIntent` | `PAYMENT_INTENT_FAILED` |
| `Failed` / `FailedAllRoutes` | `VoidPaymentIntent` | `PAYMENT_INTENT_FAILED` |
| `Voided` | `CapturePaymentIntent` | `PAYMENT_INTENT_VOIDED` |
| `Voided` | `RefundPaymentIntent` | `PAYMENT_INTENT_VOIDED` |
| `AuthorizationExpired` | `CapturePaymentIntent` | `AUTHORIZATION_EXPIRED` |
| `AuthorizationExpired` | `VoidPaymentIntent` | `AUTHORIZATION_EXPIRED` |
| `Captured` (full) | `CapturePaymentIntent` | `PAYMENT_INTENT_ALREADY_CAPTURED` |
| `Captured` | `AuthorizePaymentIntent` | `PAYMENT_INTENT_ALREADY_CAPTURED` |
| `Refunded` (full) | `RefundPaymentIntent` | `PAYMENT_INTENT_FULLY_REFUNDED` |
| `Refunded` | `CapturePaymentIntent` | `PAYMENT_INTENT_FULLY_REFUNDED` |
| `Authorizing` | `CapturePaymentIntent` | `PAYMENT_INTENT_AUTHORIZING` |
| `Authorizing` | `VoidPaymentIntent` | `PAYMENT_INTENT_AUTHORIZING` |
| `Capturing` | `VoidPaymentIntent` | `PAYMENT_INTENT_CAPTURING` |
| `Capturing` | `RefundPaymentIntent` | `PAYMENT_INTENT_CAPTURING` |
| `Created` | `CapturePaymentIntent` | `PAYMENT_INTENT_NOT_AUTHORIZED` |
| `Created` | `VoidPaymentIntent` | `PAYMENT_INTENT_NOT_AUTHORIZED` |
| `Created` | `RefundPaymentIntent` | `PAYMENT_INTENT_NOT_AUTHORIZED` |

**Invariants**:
- **INV-01**: `captured_amount <= authorized_amount` (enforced via optimistic concurrency)
- **INV-01a**: Sum of all partial captures + final capture never exceeds authorized amount
- **INV-01b**: When `supports_partial_capture = false`, capture must request full authorized amount
- **INV-01c**: Max partial captures per connector configurable (default: 10)
- **INV-02**: No two `Authorized`-outcome routing attempts simultaneously
- **INV-03**: Refunds target the same acquirer that captured
- **INV-04**: Every state transition caused by exactly one domain event
- **INV-10**: Double-entry ledger balanced (debit = credit per transaction_id)

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_intent")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub status: String,
    pub requested_amount_minor_units: i64,
    pub authorized_amount_minor_units: i64,
    pub captured_amount_minor_units: i64,
    pub refunded_amount_minor_units: i64,
    pub currency: String,
    pub idempotency_key: String,
    pub payment_method_token_id: Option<Uuid>,
    pub routing_policy_id: Option<Uuid>,
    pub deployment_epoch: i32,
    pub purpose: String, // 'payment' | 'card_verification'
    pub metadata: Option<String>, // JSON
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
```

#### AGG-02: RoutingPolicy (Aggregate Root)

**Identity**: `routing_policy_id: Uuid` (UUIDv7)

**Entities**: `RoutingRule` (ordered, versioned)

**Value Objects**: `RoutingCondition`, `FailoverConfig`, `PartialAuthorizationPolicy`

**Invariant (INV-05)**: Policy version is immutable once activated; changes create new version.

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "routing_policy")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub routing_policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub status: String, // 'active' | 'inactive'
    pub rules_json: String, // JSON array of RoutingRule
    pub failover_config_json: String, // JSON FailoverConfig
    pub partial_auth_policy_json: String,
    pub max_transaction_amount_minor_units: Option<i64>,
    pub created_at: DateTimeWithTimeZone,
    pub activated_at: Option<DateTimeWithTimeZone>,
}
```

---

## 2. Commands

### CreatePaymentIntent

```rust
pub struct CreatePaymentIntentCommand {
    pub idempotency_key: IdempotencyKey,
    pub amount: Money,
    pub purpose: PaymentPurpose, // Payment | CardVerification
    pub metadata: Option<serde_json::Value>,
}

pub enum PaymentPurpose {
    Payment,
    CardVerification, // zero-amount auth
}
```

**Preconditions**:
- Valid `Money` (amount >= 0, valid currency)
- `idempotency_key` not previously used for a *different* payload
- Duplicate key with same payload → return existing result (idempotent replay)

**Produces**: `PaymentIntentCreated` (EVT-01)

**TDD Test Cases**:

```rust
#[tokio::test]
async fn test_create_payment_intent_success() {
    let cmd = CreatePaymentIntentCommand {
        idempotency_key: "key-001".into(),
        amount: Money { amount_minor_units: 10000, currency: CurrencyCode::new("AED").unwrap() },
        purpose: PaymentPurpose::Payment,
        metadata: None,
    };
    let result = handler.handle(cmd).await.unwrap();
    assert_eq!(result.status, PaymentStatus::Created);
    assert_eq!(result.amount, 10000);
}

#[tokio::test]
async fn test_create_payment_intent_idempotent_replay() {
    let cmd = CreatePaymentIntentCommand {
        idempotency_key: "key-002".into(),
        amount: Money { amount_minor_units: 5000, currency: CurrencyCode::new("AED").unwrap() },
        purpose: PaymentPurpose::Payment,
        metadata: None,
    };
    let r1 = handler.handle(cmd.clone()).await.unwrap();
    let r2 = handler.handle(cmd).await.unwrap();
    assert_eq!(r1.payment_intent_id, r2.payment_intent_id); // same result
}

#[tokio::test]
async fn test_create_payment_intent_idempotency_conflict() {
    let cmd1 = CreatePaymentIntentCommand {
        idempotency_key: "key-003".into(),
        amount: Money { amount_minor_units: 5000, currency: CurrencyCode::new("AED").unwrap() },
        purpose: PaymentPurpose::Payment,
        metadata: None,
    };
    let cmd2 = CreatePaymentIntentCommand {
        idempotency_key: "key-003".into(),
        amount: Money { amount_minor_units: 9999, currency: CurrencyCode::new("AED").unwrap() }, // different!
        purpose: PaymentPurpose::Payment,
        metadata: None,
    };
    handler.handle(cmd1).await.unwrap();
    let result = handler.handle(cmd2).await;
    assert!(matches!(result, Err(PlatformError::Conflict(ConflictError::IdempotencyKeyConflict))));
}

#[tokio::test]
async fn test_create_payment_intent_zero_amount_card_verification() {
    let cmd = CreatePaymentIntentCommand {
        idempotency_key: "key-004".into(),
        amount: Money { amount_minor_units: 0, currency: CurrencyCode::new("AED").unwrap() },
        purpose: PaymentPurpose::CardVerification,
        metadata: None,
    };
    let result = handler.handle(cmd).await.unwrap();
    assert!(result.amount.is_zero());
}
```

### AuthorizePaymentIntent

```rust
pub struct AuthorizePaymentIntentCommand {
    pub payment_intent_id: PaymentIntentId,
    pub payment_method_token_id: Uuid,
}
```

**Preconditions**:
- `PaymentIntent` in `Created` or `Failed` (mid-retry) state
- Active `RoutingPolicy` exists
- `deployment_epoch` matches current epoch (PAY-DEPLOY-001)

**Produces**: `PaymentAuthorizationAttempted` (EVT-02), then `PaymentAuthorized` (EVT-03) or `PaymentFailed` (EVT-06)

**Routing Algorithm**:
1. Load active `RoutingPolicy`
2. Filter rules by card scheme, currency, amount
3. Map to active `MerchantAcquirerLink` IDs
4. Exclude already-attempted links
5. Order by priority
6. Select first candidate

**Failover**:
- On retryable decline → immediately attempt next candidate
- Max hops: platform ceiling 3 (RTY-002)
- Each hop recorded as `PaymentAuthorizationAttempted` event

**TDD Test Cases**:

```rust
#[tokio::test]
async fn test_authorize_single_hop_success() {
    // Setup: PaymentIntent in Created, RoutingPolicy with 1 acquirer
    // Mock acquirer returns Approved
    let result = handler.handle(authorize_cmd).await.unwrap();
    assert_eq!(result.status, PaymentStatus::Authorized);
    assert_eq!(result.attempts.len(), 1);
    assert!(result.attempts[0].approved);
}

#[tokio::test]
async fn test_authorize_failover_on_retryable_decline() {
    // Setup: PaymentIntent in Created, RoutingPolicy with 2 acquirers
    // Mock acquirer A returns Declined(InsufficientFunds) — retryable
    // Mock acquirer B returns Approved
    let result = handler.handle(authorize_cmd).await.unwrap();
    assert_eq!(result.status, PaymentStatus::Authorized);
    assert_eq!(result.attempts.len(), 2);
    assert!(!result.attempts[0].approved);
    assert!(result.attempts[1].approved);
}

#[tokio::test]
async fn test_authorize_failover_exhausted() {
    // Setup: RoutingPolicy with 1 acquirer (only one)
    // Mock acquirer returns Declined(InvalidCard) — not retryable
    let result = handler.handle(authorize_cmd).await.unwrap();
    assert_eq!(result.status, PaymentStatus::FailedAllRoutes);
}

#[tokio::test]
async fn test_authorize_no_eligible_route() {
    // Setup: No active acquirer links
    let result = handler.handle(authorize_cmd).await;
    assert!(matches!(result, Err(PlatformError::Conflict(_))));
}

#[tokio::test]
async fn test_authorize_same_acquirer_not_retried() {
    // Setup: RoutingPolicy with 1 acquirer (attempted)
    // Verify: second attempt on same acquirer is skipped
    // This is enforced by filtering attempted_hops in routing algorithm
}

#[tokio::test]
async fn test_authorize_partial_authorization_retry_next() {
    // Mock acquirer returns partial auth (approved for less than requested)
    // Default policy: retry next acquirer
    let result = handler.handle(authorize_cmd).await.unwrap();
    assert_eq!(result.attempts.len(), 2); // first partial, second full
}
```

### CapturePaymentIntent

```rust
pub struct CapturePaymentIntentCommand {
    pub payment_intent_id: PaymentIntentId,
    pub amount: Option<Money>, // None = full capture
}
```

**Preconditions**:
- `PaymentIntent` in `Authorized` or `PartiallyCaptured` state
- Requested amount <= remaining authorized amount (INV-01)
- `supports_partial_capture` checked against connector capabilities

**Produces**: `PaymentCaptured` (EVT-04) or `PaymentPartiallyCaptured` (EVT-05)

**TDD Test Cases**:

```rust
#[tokio::test]
async fn test_capture_full_amount() {
    // PaymentIntent authorized for 10000 AED
    let result = handler.handle(CapturePaymentIntentCommand {
        payment_intent_id,
        amount: None, // full capture
    }).await.unwrap();
    assert_eq!(result.status, PaymentStatus::Captured);
    assert_eq!(result.captured_amount, 10000);
}

#[tokio::test]
async fn test_capture_partial_amount() {
    // PaymentIntent authorized for 10000 AED
    let result = handler.handle(CapturePaymentIntentCommand {
        payment_intent_id,
        amount: Some(Money { amount_minor_units: 3000, currency: AED }),
    }).await.unwrap();
    assert_eq!(result.status, PaymentStatus::PartiallyCaptured);
    assert_eq!(result.captured_amount, 3000);
}

#[tokio::test]
async fn test_capture_exceeds_authorized_rejected() {
    // PaymentIntent authorized for 5000 AED
    let result = handler.handle(CapturePaymentIntentCommand {
        payment_intent_id,
        amount: Some(Money { amount_minor_units: 6000, currency: AED }),
    }).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}

#[tokio::test]
async fn test_capture_already_captured_rejected() {
    // PaymentIntent already fully captured
    let result = handler.handle(capture_cmd).await;
    assert!(matches!(result, Err(PlatformError::Conflict(ConflictError::AlreadyCaptured))));
}

#[tokio::test]
async fn test_capture_on_failed_intent_rejected() {
    // PaymentIntent in Failed state
    let result = handler.handle(capture_cmd).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}
```

### VoidPaymentIntent

```rust
pub struct VoidPaymentIntentCommand {
    pub payment_intent_id: PaymentIntentId,
}
```

**Preconditions**:
- `PaymentIntent` in `Authorized` state (not yet captured)

**Produces**: `PaymentVoided` (EVT-08)

**TDD Test Cases**:

```rust
#[tokio::test]
async fn test_void_authorized_intent() {
    let result = handler.handle(VoidPaymentIntentCommand { payment_intent_id }).await.unwrap();
    assert_eq!(result.status, PaymentStatus::Voided);
}

#[tokio::test]
async fn test_void_after_capture_rejected() {
    // PaymentIntent in Captured state
    let result = handler.handle(VoidPaymentIntentCommand { payment_intent_id }).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}

#[tokio::test]
async fn test_void_already_voided_rejected() {
    // PaymentIntent in Voided state
    let result = handler.handle(VoidPaymentIntentCommand { payment_intent_id }).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}
```

### RefundPaymentIntent

```rust
pub struct RefundPaymentIntentCommand {
    pub payment_intent_id: PaymentIntentId,
    pub amount: Money,
}
```

**Preconditions**:
- `PaymentIntent` in `Captured` or `PartiallyCaptured` state
- Amount <= remaining refundable balance (captured - refunded)
- Same acquirer as original capture (INV-03)
- Acquirer link still `Active` (REFUND-EDGE-001)
- Amount > 0 (REFUND-EDGE-003: zero-amount auths cannot be refunded)
- Above threshold → Maker/Checker required (MKCK-001)

**Produces**: `PaymentRefunded` (EVT-09) or `PaymentPartiallyRefunded` (EVT-10)

**Concurrency**: `SELECT FOR UPDATE` on PaymentIntent before balance check (REFUND-CONC-001)

**TDD Test Cases**:

```rust
#[tokio::test]
async fn test_refund_full_amount() {
    let result = handler.handle(RefundPaymentIntentCommand {
        payment_intent_id,
        amount: Money { amount_minor_units: 10000, currency: AED },
    }).await.unwrap();
    assert_eq!(result.status, PaymentStatus::Refunded);
}

#[tokio::test]
async fn test_refund_partial_amount() {
    let result = handler.handle(RefundPaymentIntentCommand {
        payment_intent_id,
        amount: Money { amount_minor_units: 3000, currency: AED },
    }).await.unwrap();
    assert_eq!(result.status, PaymentStatus::PartiallyRefunded);
    assert_eq!(result.refunded_amount, 3000);
}

#[tokio::test]
async fn test_refund_exceeds_balance_rejected() {
    // Captured 5000, try refund 6000
    let result = handler.handle(refund_cmd).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}

#[tokio::test]
async fn test_refund_zero_amount_rejected() {
    // Zero-amount authorization cannot be refunded
    let result = handler.handle(refund_cmd).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}

#[tokio::test]
async fn test_refund_disabled_acquirer_rejected() {
    // Original acquirer link is Disabled
    let result = handler.handle(refund_cmd).await;
    assert!(matches!(result, Err(PlatformError::Conflict(_))));
}

#[tokio::test]
async fn test_concurrent_refunds_prevented() {
    // Two concurrent refund requests totaling more than captured
    // First should succeed, second should fail with balance exceeded
    // This is tested via optimistic concurrency + SELECT FOR UPDATE
}
```

### ActivateRoutingPolicy

```rust
pub struct ActivateRoutingPolicyCommand {
    pub rules: Vec<RoutingRule>,
    pub failover_config: FailoverConfig,
    pub partial_auth_policy: PartialAuthorizationPolicy,
    pub max_transaction_amount: Option<Money>,
}
```

**Preconditions**:
- No circular references in rules
- All referenced acquirer links are `Active`
- Maker/Checker approval required (MKCK-001)

**Produces**: `RoutingPolicyActivated` (EVT-11), previous policy's `RoutingPolicyDeactivated` (EVT-12)

---

## 3. Domain Events

| Event | Fields | Published To |
|---|---|---|
| `PaymentIntentCreated` | payment_intent_id, amount, currency, purpose | NATS |
| `PaymentAuthorizationAttempted` | payment_intent_id, acquirer_link_id, routing_policy_version, decline_reason, latency_ms | NATS |
| `PaymentAuthorized` | payment_intent_id, acquirer_reference, acquirer_link_id | NATS |
| `PaymentCaptured` | payment_intent_id, captured_amount | NATS |
| `PaymentPartiallyCaptured` | payment_intent_id, captured_amount, remaining_authorized | NATS |
| `PaymentFailed` | payment_intent_id, acquirer_link_id, decline_reason | NATS |
| `PaymentFailedAllRoutes` | payment_intent_id, attempts: Vec<RoutingAttemptResult> | NATS |
| `PaymentVoided` | payment_intent_id | NATS |
| `PaymentRefunded` | payment_intent_id, refund_amount, acquirer_reference | NATS |
| `PaymentPartiallyRefunded` | payment_intent_id, refund_amount, remaining_refundable | NATS |
| `RoutingPolicyActivated` | routing_policy_id, version, rules_hash | NATS |
| `RoutingPolicyDeactivated` | routing_policy_id, version | NATS |

---

## 4. Repository Interface

```rust
#[async_trait]
pub trait PaymentIntentRepository: Send + Sync {
    async fn load(&self, id: PaymentIntentId) -> Result<Option<PaymentIntent>, PlatformError>;
    async fn save(&self, aggregate: &PaymentIntent, expected_version: i64) -> Result<(), PlatformError>;
    async fn load_events(&self, id: PaymentIntentId) -> Result<Vec<EventEnvelope>, PlatformError>;
    async fn append_events(&self, id: PaymentIntentId, events: &[EventEnvelope], expected_version: i64) -> Result<(), PlatformError>;
}

#[async_trait]
pub trait RoutingPolicyRepository: Send + Sync {
    async fn load_active(&self, operator_id: OperatorId) -> Result<Option<RoutingPolicy>, PlatformError>;
    async fn save(&self, policy: &RoutingPolicy) -> Result<(), PlatformError>;
}

#[async_trait]
pub trait IdempotencyCache: Send + Sync {
    async fn check_and_store(&self, key: &IdempotencyKey, payload_hash: &[u8]) -> Result<IdempotencyResult, PlatformError>;
}

pub enum IdempotencyResult {
    New,
    Duplicate { result: serde_json::Value },
    Conflict, // same key, different payload
}
```

---

## 5. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `PAYMENT_INTENT_NOT_FOUND` | 404 | NOT_FOUND | PaymentIntent does not exist |
| `INVALID_STATE_TRANSITION` | 409 | FAILED_PRECONDITION | Command not valid for current state |
| `PAYMENT_INTENT_FAILED` | 409 | FAILED_PRECONDITION | In terminal failed state |
| `PAYMENT_INTENT_VOIDED` | 409 | FAILED_PRECONDITION | Has been voided |
| `AUTHORIZATION_EXPIRED` | 409 | FAILED_PRECONDITION | Auth window passed |
| `PAYMENT_INTENT_ALREADY_CAPTURED` | 409 | FAILED_PRECONDITION | Fully captured |
| `PAYMENT_INTENT_FULLY_REFUNDED` | 409 | FAILED_PRECONDITION | Fully refunded |
| `PAYMENT_INTENT_AUTHORIZING` | 409 | FAILED_PRECONDITION | Currently authorizing |
| `PAYMENT_INTENT_CAPTURING` | 409 | FAILED_PRECONDITION | Currently capturing |
| `PAYMENT_INTENT_NOT_AUTHORIZED` | 409 | FAILED_PRECONDITION | Not yet authorized |
| `INSUFFICIENT_AUTHORIZED_AMOUNT` | 400 | INVALID_ARGUMENT | Capture exceeds auth |
| `INSUFFICIENT_REFUNDABLE_BALANCE` | 400 | INVALID_ARGUMENT | Refund exceeds balance |
| `ACQUIRER_LINK_DISABLED` | 409 | FAILED_PRECONDITION | Original acquirer disabled |
| `ZERO_AMOUNT_NOT_REFUNDABLE` | 400 | INVALID_ARGUMENT | Can't refund card verification |
| `PARTIAL_CAPTURE_NOT_SUPPORTED` | 400 | INVALID_ARGUMENT | Connector doesn't support |
| `MAX_PARTIAL_CAPTURES_EXCEEDED` | 400 | INVALID_ARGUMENT | Too many partial captures |
| `NO_ELIGIBLE_ROUTE` | 422 | FAILED_PRECONDITION | No acquirer matches |
| `ROUTING_POLICY_INACTIVE` | 409 | FAILED_PRECONDITION | No active policy |
| `ALL_ACQUIRERS_DECLINED` | 402 | FAILED_PRECONDITION | All hops failed |
| `DUPLICATE_IDEMPOTENCY_KEY` | 409 | ALREADY_EXISTS | Key with different payload |
| `CONCURRENCY_VIOLATION` | 409 | ABORTED | Optimistic concurrency failed |
