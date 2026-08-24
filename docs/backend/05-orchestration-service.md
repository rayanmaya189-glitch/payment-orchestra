# 05 — orchestration-service (BC-05 Payment Orchestration)

> ⚡ **Pure Router**: The platform is a routing and orchestration layer only. Funds flow directly between the customer, the payment gateway, and the merchant bank account. The platform never holds, touches, or controls funds.

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
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
Created → Authorizing → [3DS Required → 3DS Authenticating →] Authorized → Capturing → Captured
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

    // Source Context — who initiated this payment (Gap: analytics segmentation)
    pub source_type: Option<String>,        // 'merchant_api' | 'invoice' | 'subscription' | 'payment_link' | 'ai_assistant' | 'system'
    pub source_id: Option<Uuid>,            // invoice_id, subscription_id, payment_link_id, etc.

    // Risk Score — pre-authorization risk assessment (Gap: risk integration)
    pub risk_score: Option<f64>,            // 0.0 - 1.0, set before authorization
    pub risk_level: Option<String>,         // 'low' | 'medium' | 'high' | 'critical'

    // Settlement Timing — expected settlement date (Gap: T+N handling)
    pub expected_settlement_date: Option<Date>,
    pub settlement_cycle: Option<String>,   // 'same_day' | 'next_day' | 'two_days' | 'three_days' | 'weekly'

    // Gateway Profile Link — tracks which gateway handled this order
    pub gateway_profile_id: Option<Uuid>,           // which gateway profile was selected
    pub gateway_profile_version: Option<i32>,        // snapshot of profile at selection time
    pub gateway_rotation_strategy: Option<String>,   // rotation strategy used
    pub gateway_selection_reason: Option<String>,     // 'priority_1', 'round_robin_2', etc.

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
```

### RoutingAttempt — Extended with Gateway Profile

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "routing_attempt")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub attempt_id: Uuid,
    pub payment_intent_id: Uuid,
    pub attempt_number: i32,
    pub acquirer_link_id: Uuid,
    pub gateway_profile_id: Uuid,            // gateway profile used for this attempt
    pub gateway_profile_snapshot: String,    // JSON snapshot of profile at attempt time
    pub connector_id: String,
    pub status: String,                      // 'approved' | 'declined' | 'timeout'
    pub decline_reason: Option<String>,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
    pub fee_calculated: Option<String>,      // JSON FeeBreakdown
    pub attempted_at: DateTimeWithTimeZone,
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

#### AGG-03: PaymentMethodToken (Aggregate Root) — Gap: Token Lifecycle

**Identity**: `token_id: Uuid` (UUIDv7)

**Purpose**: Manage acquirer-issued payment method tokens for recurring payments. The platform never stores raw card data — tokens are opaque references issued by acquirers.

**Invariant (INV-06)**: A token is scoped to a specific `MerchantAcquirerLink` and cannot be used with a different acquirer.

**Invariant (INV-07)**: A token in `Revoked` or `Expired` state cannot be used to create new `PaymentIntent`s.

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_method_token")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub token_id: Uuid,
    pub operator_id: Uuid,
    pub payment_method_type: String,    // 'card' | 'bank_account' | 'wallet'
    pub last_four: String,
    pub card_brand: Option<String>,     // 'visa' | 'mastercard' | 'amex' | 'mada'
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub token_status: String,           // 'active' | 'expired' | 'revoked'
    pub acquirer_link_id: Uuid,         // FK to MerchantAcquirerLink
    pub acquirer_token_reference: String, // opaque token from acquirer
    pub encrypted_token: Vec<u8>,       // envelope-encrypted (PCI-DSS: token = cardholder data per OQ-089)
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub revocation_reason: Option<String>,
}
```

**Commands**:
- `StorePaymentMethodToken` — store acquirer-issued token after successful tokenization
- `ExpirePaymentMethodToken` — mark token as expired (card expiry or account updater)
- `RevokePaymentMethodToken` — revoke token (merchant request or security concern)
- `RefreshPaymentMethodToken` — account-updater: acquirer notifies of card refresh

**Domain Events**: `PaymentMethodTokenStored`, `PaymentMethodTokenExpired`, `PaymentMethodTokenRevoked`

**Repository Interface**:

```rust
#[async_trait]
pub trait PaymentMethodTokenRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentMethodToken>, PlatformError>;
    async fn save(&self, token: &PaymentMethodToken) -> Result<(), PlatformError>;
    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentMethodToken>, PlatformError>;
    async fn find_by_acquirer_link(&self, acquirer_link_id: Uuid) -> Result<Vec<PaymentMethodToken>, PlatformError>;
    async fn find_by_acquirer_reference(&self, acquirer_token_ref: &str) -> Result<Option<PaymentMethodToken>, PlatformError>;
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
    pub preferred_gateway_profile_id: Option<Uuid>, // optional: force specific gateway
    // Gap: source context for analytics segmentation
    pub source_type: SourceType,                     // who initiated this payment
    pub source_id: Option<Uuid>,                     // invoice_id, subscription_id, etc.
    // Gap: payment method token for recurring payments
    pub payment_method_token_id: Option<Uuid>,       // acquirer-issued token for recurring
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
- If `preferred_gateway_profile_id` is specified, validate gateway is active and within limits

**Produces**: `PaymentIntentCreated` (EVT-01) with `gateway_profile_id` set

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

**Routing Algorithm (with Gateway Profile Selection and Risk Integration)**:
1. Load active `RoutingPolicy`
2. Filter rules by card scheme, currency, amount
3. Map to active `MerchantAcquirerLink` IDs
4. Load `GatewayProfile` for each candidate link
5. Filter by gateway profile limits (min/max amount, daily/monthly volume, card scheme, currency)
6. Filter by gateway profile status (active only)
7. **[Gap: Risk Check]** If `risk-service` enabled for tenant (OQ-009), call `AssessRisk` synchronously. If risk_score > operator-configured threshold, apply risk-based routing rule:
   - Route to acquirer with best fraud screening (configured per `GatewayProfile`)
   - Or reject with `HIGH_RISK_DECLINED` if no suitable acquirer
8. Apply gateway rotation strategy (priority, round-robin, cost-based, etc.)
9. **[Gap: Risk-Weighted Routing]** For `SuccessRateBased` and `CostBased` strategies, weight by (authorization_rate × (1 - risk_penalty)) where risk_penalty is derived from historical fraud rates per acquirer
10. Exclude already-attempted links
11. Select first eligible candidate
12. Calculate fee for selected gateway profile
13. **[Gap: Settlement Timing]** Calculate `expected_settlement_date` from acquirer's `settlement_cycle` on `GatewayProfile`
14. Record `gateway_profile_id`, `source_type`, `source_id`, `risk_score`, `expected_settlement_date` on `PaymentIntent` and `RoutingAttempt`

### Order-Gateway Profile Linking Flow

```
1. CreatePaymentIntent → gateway_profile_id NOT yet set (selected at authorize time)
2. AuthorizePaymentIntent:
   a. Load all eligible GatewayProfiles for this operator
   b. Apply rotation strategy to select best profile
   c. Set PaymentIntent.gateway_profile_id = selected profile
   d. Set PaymentIntent.gateway_rotation_strategy = strategy used
   e. Set PaymentIntent.gateway_selection_reason = reason text
   f. Create RoutingAttempt with gateway_profile_id + snapshot
3. On failover (next hop):
   a. Load next eligible GatewayProfile (excluding attempted gateways)
   b. Create new RoutingAttempt with new gateway_profile_id
   c. Update PaymentIntent.routing_attempt_gateway_ids
4. On completion:
   a. PaymentIntent.gateway_profile_id = winning gateway (the one that approved)
   b. Fee calculated from winning gateway's FeeStructure
   c. GatewayProfileSelected event emitted with full audit trail
```

**Audit Trail**:

```rust
pub struct GatewayProfileSelected {
    pub payment_intent_id: Uuid,
    pub gateway_profile_id: Uuid,
    pub connector_id: String,
    pub rotation_strategy: String,
    pub selection_reason: String,
    pub fee_calculated: Money,
    pub daily_volume_after: Money,
    pub monthly_volume_after: Money,
    pub occurred_at: DateTime<Utc>,
}
```

```rust
// Enhanced routing with gateway profiles
pub async fn select_route_with_profiles(
    intent: &PaymentIntent,
    policy: &RoutingPolicy,
    links: &[MerchantAcquirerLink],
    profiles: &[GatewayProfile],
    attempted_hops: &[Uuid],
) -> Result<RouteSelection, PlatformError> {
    let mut candidates: Vec<(Uuid, i32)> = vec![];

    for rule in &policy.rules {
        // 1. Match rule conditions
        if !rule.matches(intent.card_scheme, intent.currency, intent.amount) {
            continue;
        }

        // 2. Find matching link
        let link = links.iter()
            .find(|l| l.id == rule.acquirer_link_id && l.status == "active")
            .ok_or(PlatformError::NotFound)?;

        // 3. Find gateway profile
        let profile = profiles.iter()
            .find(|p| p.merchant_acquirer_link_id == link.id && p.status == "active")
            .ok_or(PlatformError::NotFound)?;

        // 4. Validate against gateway profile limits
        validate_transaction_against_profile(
            &intent.amount, profile, &intent.card_scheme, &intent.currency,
        )?;

        // 5. Check daily volume
        let daily_volume = profile_repo.check_daily_volume(profile.id).await?;
        if daily_volume.amount_minor_units + intent.amount.amount_minor_units > profile.daily_volume_limit_minor {
            continue; // Skip this gateway — daily limit reached
        }

        // 6. Exclude attempted
        if attempted_hops.contains(&link.id) {
            continue;
        }

        candidates.push((link.id, profile.routing_priority));
    }

    // 7. Sort by priority
    candidates.sort_by_key(|(_, priority)| *priority);

    candidates.first()
        .map(|(id, _)| RouteSelection::Acquirer(*id))
        .ok_or(PlatformError::Conflict(ConflictError::NoEligibleRoute))
}
```

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

**TDD Tests**:

```rust
#[tokio::test]
async fn test_activate_routing_policy_success() {
    let result = handler.handle(ActivateRoutingPolicyCommand {
        rules: vec![RoutingRule { acquirer_link_id, priority: 1, condition: RoutingCondition::all() }],
        failover_config: FailoverConfig::default(),
        partial_auth_policy: PartialAuthorizationPolicy { strategy: PartialAuthStrategy::RetryNextAcquirer },
        max_transaction_amount: None,
    }).await.unwrap();
    assert_eq!(result.status, "active");
    assert_eq!(result.version, 1);
}

#[tokio::test]
async fn test_activate_routing_policy_immutability() {
    // Policy v1 activated
    let v1 = handler.handle(ActivateRoutingPolicyCommand { ... }).await.unwrap();
    // Try to modify v1 → rejected (INV-05)
    // Create v2 instead
    let v2 = handler.handle(ActivateRoutingPolicyCommand { ... }).await.unwrap();
    assert_eq!(v2.version, 2);
}

#[tokio::test]
async fn test_activate_routing_policy_validates_acquirer_links() {
    // Rule references a disabled acquirer link
    let result = handler.handle(ActivateRoutingPolicyCommand {
        rules: vec![RoutingRule { acquirer_link_id: disabled_link_id, ... }],
        ...
    }).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_activate_routing_policy_maker_checker_required() {
    // Without Maker/Checker approval → rejected
}
```

### Missing Invariant Tests (Added per Gap Analysis)

```rust
#[tokio::test]
async fn test_inv_01a_partial_captures_sum_never_exceeds_authorized() {
    // Authorized: 10000
    // Capture 3000 → OK
    // Capture 4000 → OK (total 7000)
    // Capture 4000 → REJECTED (total would be 11000 > 10000)
}

#[tokio::test]
async fn test_inv_01b_full_capture_required_when_partial_not_supported() {
    // Connector with supports_partial_capture = false
    // Capture 5000 of 10000 → REJECTED with PARTIAL_CAPTURE_NOT_SUPPORTED
}

#[tokio::test]
async fn test_inv_01c_max_partial_catches_per_connector() {
    // Connector with max_partial_captures = 3
    // Capture 3 times → OK
    // Capture 4th time → REJECTED with MAX_PARTIAL_CAPTURES_EXCEEDED
}

#[tokio::test]
async fn test_inv_05_routing_policy_immutable_once_activated() {
    // Activate policy v1
    // Attempt to modify rules on v1 → rejected
    // Must create v2
}

#[tokio::test]
async fn test_inv_07_settlement_match_refs_exactly_one_payment_intent() {
    // Settlement record matches 0 intents → Unmatched
    // Settlement record matches 2 intents → DuplicateReference
    // Settlement record matches 1 intent → Matched
}
```

---

## 3. Domain Events — Consumer Mapping (Complete)

| Event | Consumer 1 | Consumer 2 | Consumer 3 | Consumer 4 |
|---|---|---|---|---|
| `PaymentIntentCreated` | reconciliation-service | analytics-service | ai-assistant-service | — |
| `PaymentAuthorizationAttempted` | analytics-service | ai-assistant-service | risk-service | — |
| `PaymentAuthorized` | invoice-service | subscription-service | reconciliation-service | notification-service |
| `PaymentCaptured` | invoice-service | subscription-service | reconciliation-service | analytics-service |
| `PaymentPartiallyCaptured` | reconciliation-service | analytics-service | — | — |
| `PaymentFailed` | analytics-service | ai-assistant-service | risk-service | notification-service |
| `PaymentFailedAllRoutes` | notification-service | ai-assistant-service | risk-service | — |
| `PaymentVoided` | invoice-service | reconciliation-service | — | — |
| `PaymentRefunded` | invoice-service | reconciliation-service | notification-service | — |
| `PaymentPartiallyRefunded` | reconciliation-service | notification-service | — | — |
| `RoutingPolicyActivated` | analytics-service | ai-assistant-service | — | — |
| `RoutingPolicyDeactivated` | analytics-service | — | — | — |
| `PaymentMethodTokenStored` | subscription-service | analytics-service | — | — |
| `PaymentMethodTokenExpired` | subscription-service | notification-service | — | — |
| `PaymentMethodTokenRevoked` | subscription-service | notification-service | — | — |
| `RiskScoreAssigned` | analytics-service | ai-assistant-service | — | — |

| Event | Fields | Published To |
|---|---|---|---|
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
| `PaymentMethodTokenStored` | token_id, payment_method_type, last_four, card_brand, acquirer_link_id | NATS |
| `PaymentMethodTokenExpired` | token_id, last_four, acquirer_link_id | NATS |
| `PaymentMethodTokenRevoked` | token_id, last_four, acquirer_link_id, revocation_reason | NATS |
| `RiskScoreAssigned` | payment_intent_id, risk_score, risk_level, rule_version | NATS |

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


