# 04 — connector-gateway (BC-04 Gateway Connector Framework)

> ⚡ **Pure Router**: The platform is a routing and orchestration layer only. Funds flow directly between the customer, the payment gateway, and the merchant bank account. The platform never holds, touches, or controls funds.

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
Anti-Corruption Layer. Translates N acquirer APIs into one normalized internal protocol.

**BYOK Critical Service**: This service provides the `OnboardingSchema` and credential validation that every merchant needs to connect their own gateway credentials.

---

## 1. Core Abstraction: AcquirerConnector Trait

```rust
#[async_trait]
pub trait AcquirerConnector: Send + Sync {
    fn connector_id(&self) -> ConnectorId;
    fn capabilities(&self) -> ConnectorCapabilities;

    // Core payment operations
    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError>;
    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError>;
    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError>;
    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError>;
    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError>;

    // FX rate query — acquirer provides conversion rates for cross-border transactions
    async fn get_fx_rate(&self, req: FxRateRequest) -> Result<FxRateResponse, ConnectorError>;

    // Settlement cycle query — acquirer reports expected settlement timing
    fn settlement_cycle(&self) -> SettlementCycle;

    // NEW: 3D Secure support
    /// Check if a card/enrollment requires 3DS authentication
    async fn check_3ds_enrollment(&self, req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError>;
    /// Authenticate via 3DS (handles challenge/frictionless flow)
    async fn authenticate_3ds(&self, req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError>;

    // NEW: Network Token support
    /// Provision a network token (Visa Token Service, Mastercard MDES)
    async fn provision_network_token(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError>;

    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError>;
    fn verify_webhook_signature(&self, headers: &HeaderMap, body: &[u8]) -> Result<(), ConnectorError>;
    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError>;

    // BYOK: Onboarding & Credential Management
    /// Dynamic credential schema for frontend form rendering
    fn onboarding_schema(&self) -> OnboardingSchema;
    /// Validate credentials via sandbox/status-check (never live-money)
    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError>;
    /// Full connection test — returns merchant name, permissions, latency
    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError>;
    /// Get test card numbers valid for this connector's sandbox
    fn test_card_numbers(&self) -> Vec<TestCardNumber>;

    // NEW: Account Updater (for recurring/subscription payments)
    async fn account_updater(&self, token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError>;
}
```

## 2. Gateway Profile & Limits (Per-Connector Configuration)

Each connected acquirer/PSP has a **Gateway Profile** that defines operational limits, fee structure, and routing preferences.

### Gateway Profile Entity

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gateway_profile")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub profile_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,             // 'network_international' | 'checkout_com' | 'telr'
    pub merchant_acquirer_link_id: Uuid,  // FK to MerchantAcquirerLink
    pub status: String,                   // 'active' | 'disabled' | 'maintenance'

    // Transaction Limits
    pub min_transaction_amount_minor: i64,    // e.g., 100 (1.00 AED)
    pub max_transaction_amount_minor: i64,    // e.g., 50000000 (500,000 AED)
    pub daily_volume_limit_minor: i64,        // e.g., 5000000000 (50,000,000 AED)
    pub monthly_volume_limit_minor: i64,      // e.g., 50000000000 (500,000,000 AED)
    pub max_refund_amount_minor: i64,         // per-transaction refund cap

    // Fee Structure
    pub fixed_fee_minor: i64,                 // e.g., 100 (1.00 AED per transaction)
    pub percentage_fee_bps: i32,              // basis points, e.g., 250 = 2.50%
    pub cross_border_fee_bps: i32,            // additional fee for cross-border
    pub currency_conversion_fee_bps: i32,     // additional fee for FX

    // Routing Preferences
    pub routing_priority: i32,                // 1 = highest, used in RoutingPolicy
    pub enabled_card_schemes: String,         // JSON array: ["visa", "mastercard"]
    pub enabled_currencies: String,           // JSON array: ["AED", "USD"]
    pub enabled_countries: String,            // JSON array: ["AE", "SA", "BH"]

    // Rate Limiting (per-connector)
    pub rate_limit_per_second: u32,           // max API calls per second to this acquirer
    pub rate_limit_per_day: u32,              // max API calls per day

    // Monitoring
    pub success_rate_threshold: f64,          // alert if success rate drops below (e.g., 0.95)
    pub latency_threshold_ms: u32,            // alert if p99 latency exceeds (e.g., 5000)
    pub auto_disable_on_low_success: bool,    // auto-disable if success rate < threshold for 1hr

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
```

### Gateway Profile Value Objects

```rust
pub struct TransactionLimits {
    pub min_amount: Money,
    pub max_amount: Money,
    pub daily_volume: Money,
    pub monthly_volume: Money,
    pub max_refund_amount: Money,
}

pub struct FeeStructure {
    pub fixed_fee: Money,
    pub percentage_fee_bps: i32,       // basis points
    pub cross_border_fee_bps: i32,
    pub currency_conversion_fee_bps: i32,
    // Gap: fee caps and minimums for accurate cost calculation
    pub max_fee_cap: Option<Money>,    // maximum total fee (cap)
    pub min_fee_floor: Option<Money>,  // minimum total fee (floor)
    pub tiered_pricing: Option<Vec<FeeTier>>, // tiered pricing by volume
}

// Gap: tiered fee pricing
pub struct FeeTier {
    pub min_volume_minor: i64,         // tier lower bound (daily volume)
    pub max_volume_minor: Option<i64>, // tier upper bound (None = unlimited)
    pub percentage_fee_bps: i32,       // fee for this tier
}

impl FeeStructure {
    /// Calculate total fee for a transaction (Gap: with caps and tiers)
    pub fn calculate_fee(&self, amount: &Money, is_cross_border: bool, requires_fx: bool, daily_volume: i64) -> Money {
        let percentage_fee = (amount.amount_minor_units * self.percentage_fee_bps as i64) / 10000;
        let cross_border = if is_cross_border {
            (amount.amount_minor_units * self.cross_border_fee_bps as i64) / 10000
        } else { 0 };
        let fx_fee = if requires_fx {
            (amount.amount_minor_units * self.currency_conversion_fee_bps as i64) / 10000
        } else { 0 };

        let mut total = self.fixed_fee.amount_minor_units + percentage_fee + cross_border + fx_fee;

        // Gap: apply tiered pricing if configured
        if let Some(tiers) = &self.tiered_pricing {
            if let Some(tier) = tiers.iter().find(|t| {
                daily_volume >= t.min_volume_minor
                    && t.max_volume_minor.map_or(true, |max| daily_volume < max)
            }) {
                total = self.fixed_fee.amount_minor_units
                    + (amount.amount_minor_units * tier.percentage_fee_bps as i64) / 10000
                    + cross_border + fx_fee;
            }
        }

        // Gap: apply fee cap
        if let Some(cap) = &self.max_fee_cap {
            total = total.min(cap.amount_minor_units);
        }

        // Gap: apply fee floor
        if let Some(floor) = &self.min_fee_floor {
            total = total.max(floor.amount_minor_units);
        }

        Money {
            amount_minor_units: total,
            currency: amount.currency.clone(),
        }
    }

    /// Gap: determine if a transaction is cross-border
    /// Cross-border = card issuing country ≠ acquirer country
    pub fn is_cross_border(&self, card_issuer_country: &str, acquirer_country: &str) -> bool {
        card_issuer_country != acquirer_country
    }
}

pub struct RateLimitConfig {
    pub per_second: u32,
    pub per_day: u32,
    pub burst_size: u32, // max concurrent requests
}

pub struct MonitoringThresholds {
    pub success_rate_alert: f64,       // alert threshold
    pub success_rate_critical: f64,    // critical threshold (auto-disable)
    pub latency_p99_alert_ms: u32,
    pub latency_p99_critical_ms: u32,
}
```

### Gateway Profile Default Values per Connector

| Connector | Min Amount | Max Amount | Daily Volume | Fixed Fee | Percentage Fee | Rate Limit/sec |
|-----------|-----------|-----------|-------------|-----------|---------------|----------------|
| Network International | 1.00 AED | 500,000 AED | 50M AED | 1.00 AED | 2.50% | 100 |
| Checkout.com | 1.00 AED | 1,000,000 AED | 100M AED | 0.50 AED | 2.25% | 200 |
| Telr | 1.00 AED | 250,000 AED | 25M AED | 1.50 AED | 2.75% | 50 |

### Gateway Profile Commands

```rust
pub struct CreateGatewayProfileCommand {
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub min_transaction_amount: Money,
    pub max_transaction_amount: Money,
    pub daily_volume_limit: Money,
    pub monthly_volume_limit: Money,
    pub fixed_fee: Money,
    pub percentage_fee_bps: i32,
    pub enabled_card_schemes: Vec<CardScheme>,
    pub enabled_currencies: Vec<CurrencyCode>,
    pub routing_priority: i32,
}

pub struct UpdateGatewayProfileCommand {
    pub profile_id: Uuid,
    pub limits: Option<TransactionLimits>,
    pub fees: Option<FeeStructure>,
    pub rate_limits: Option<RateLimitConfig>,
    pub monitoring: Option<MonitoringThresholds>,
    pub status: Option<String>, // 'active' | 'disabled' | 'maintenance'
}
```

### Gateway Profile Repository

```rust
#[async_trait]
pub trait GatewayProfileRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, PlatformError>;
    async fn save(&self, profile: &GatewayProfile) -> Result<(), PlatformError>;
    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, PlatformError>;
    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, PlatformError>;
    async fn check_daily_volume(&self, profile_id: Uuid) -> Result<Money, PlatformError>;
    async fn check_monthly_volume(&self, profile_id: Uuid) -> Result<Money, PlatformError>;
}
```

### Gateway Profile Validation

```rust
pub fn validate_transaction_against_profile(
    amount: &Money,
    profile: &GatewayProfile,
    card_scheme: &CardScheme,
    currency: &CurrencyCode,
) -> Result<(), GatewayError> {
    // 1. Check amount limits
    if amount.amount_minor_units < profile.min_transaction_amount_minor {
        return Err(GatewayError::BelowMinimumAmount);
    }
    if amount.amount_minor_units > profile.max_transaction_amount_minor {
        return Err(GatewayError::ExceedsMaximumAmount);
    }

    // 2. Check daily volume
    // (validated at routing time against accumulated daily total)

    // 3. Check card scheme
    let enabled_schemes: Vec<CardScheme> = serde_json::from_str(&profile.enabled_card_schemes)?;
    if !enabled_schemes.contains(card_scheme) {
        return Err(GatewayError::UnsupportedCardScheme);
    }

    // 4. Check currency
    let enabled_currencies: Vec<CurrencyCode> = serde_json::from_str(&profile.enabled_currencies)?;
    if !enabled_currencies.contains(currency) {
        return Err(GatewayError::UnsupportedCurrency);
    }

    // 5. Check rate limit
    // (validated at request time via rate limiter)

    Ok(())
}
```

---

## 3. Gateway Profile Rotation Strategy

Each order/payment intent is linked to a specific gateway profile via a **rotation strategy** that determines which gateway handles each transaction.

### Rotation Strategy Types

```rust
pub enum RotationStrategy {
    /// Fixed priority order — always try gateway 1 first, then 2, etc.
    Priority,
    /// Round-robin — distribute evenly across gateways
    RoundRobin,
    /// Weighted round-robin — distribute by weight (e.g., 60%/40%)
    WeightedRoundRobin { weights: Vec<(Uuid, u32)> },
    /// Cost-based — select cheapest gateway for this transaction
    CostBased,
    /// Success-rate-based — select gateway with highest recent success rate
    SuccessRateBased,
    /// Volume-capped — rotate until one gateway hits daily limit, then next
    VolumeCapped,
}
```

### Rotation State (Redis)

```rust
pub struct RotationState {
    pub operator_id: Uuid,
    pub strategy: RotationStrategy,
    pub current_index: u32,           // for round-robin
    pub last_used_gateway_id: Uuid,   // for round-robin
    pub weights: Vec<(Uuid, u32)>,    // for weighted round-robin
    pub daily_volume: HashMap<Uuid, i64>, // gateway_id → volume today
}
```

### Rotation Algorithm

```rust
pub async fn select_gateway_profile(
    state: &RotationState,
    profiles: &[GatewayProfile],
    transaction: &PaymentIntent,
    routing_policy: &RoutingPolicy,
) -> Result<Uuid, PlatformError> {
    // 1. Filter profiles by active status and matching conditions
    let eligible: Vec<&GatewayProfile> = profiles.iter()
        .filter(|p| p.status == "active")
        .filter(|p| matches_card_scheme(p, transaction.card_scheme))
        .filter(|p| matches_currency(p, transaction.currency))
        .filter(|p| transaction.amount.amount_minor_units >= p.min_transaction_amount_minor)
        .filter(|p| transaction.amount.amount_minor_units <= p.max_transaction_amount_minor)
        .filter(|p| !daily_limit_exceeded(p, &state.daily_volume))
        .collect();

    if eligible.is_empty() {
        return Err(PlatformError::Conflict(ConflictError::NoEligibleRoute));
    }

    // 2. Apply rotation strategy
    match &state.strategy {
        RotationStrategy::Priority => {
            // Already sorted by routing_priority from GatewayProfile
            Ok(eligible[0].profile_id)
        }
        RotationStrategy::RoundRobin => {
            let next_index = (state.current_index as usize) % eligible.len();
            Ok(eligible[next_index].profile_id)
        }
        RotationStrategy::WeightedRoundRobin { weights } => {
            let total_weight: u32 = weights.iter().map(|(_, w)| w).sum();
            let mut random = rand::thread_rng().gen_range(0..total_weight);
            for (gateway_id, weight) in weights {
                random = random.saturating_sub(*weight);
                if random == 0 {
                    return Ok(*gateway_id);
                }
            }
            Ok(eligible[0].profile_id) // fallback
        }
        RotationStrategy::CostBased => {
            // Calculate total fee for each eligible gateway
            let mut scored: Vec<(&GatewayProfile, i64)> = eligible.iter()
                .map(|p| {
                    let fee = calculate_total_fee(p, transaction);
                    (p, fee.amount_minor_units)
                })
                .collect();
            scored.sort_by_key(|(_, fee)| *fee);
            Ok(scored[0].0.profile_id)
        }
        RotationStrategy::SuccessRateBased => {
            let mut scored: Vec<(&GatewayProfile, f64)> = eligible.iter()
                .map(|p| (p, p.success_rate))
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            Ok(scored[0].0.profile_id)
        }
        RotationStrategy::VolumeCapped => {
            // Select first gateway that hasn't hit daily limit
            for profile in &eligible {
                if !daily_limit_exceeded(profile, &state.daily_volume) {
                    return Ok(profile.profile_id);
                }
            }
            Err(PlatformError::Conflict(ConflictError::AllGatewaysVolumeExceeded))
        }
    }
}
```

### Order-Gateway Profile Link

Every `PaymentIntent` records which gateway profile was used:

```rust
// Extended PaymentIntent entity
pub struct PaymentIntent {
    // ... existing fields ...
    pub gateway_profile_id: Uuid,        // which gateway handled this order
    pub gateway_profile_version: i32,    // snapshot of profile at time of transaction
    pub routing_attempt_gateway_ids: Vec<Uuid>, // gateway used at each hop
}
```

### Gateway Profile Audit Event

```rust
pub struct GatewayProfileSelected {
    pub payment_intent_id: Uuid,
    pub gateway_profile_id: Uuid,
    pub connector_id: String,
    pub rotation_strategy: String,
    pub selection_reason: String,        // "priority_1", "round_robin_2", "lowest_cost", etc.
    pub fee_calculated: Money,
    pub daily_volume_after: Money,
}
```

---

## 4. Gateway Profile Repository

```rust
#[async_trait]
pub trait GatewayProfileRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, PlatformError>;
    async fn save(&self, profile: &GatewayProfile) -> Result<(), PlatformError>;
    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, PlatformError>;
    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, PlatformError>;
    async fn find_by_link(&self, link_id: Uuid) -> Result<Option<GatewayProfile>, PlatformError>;
    async fn check_daily_volume(&self, profile_id: Uuid) -> Result<Money, PlatformError>;
    async fn check_monthly_volume(&self, profile_id: Uuid) -> Result<Money, PlatformError>;
    async fn increment_daily_volume(&self, profile_id: Uuid, amount: Money) -> Result<(), PlatformError>;
    async fn get_success_rate(&self, profile_id: Uuid, window_hours: u32) -> Result<f64, PlatformError>;
}
```

---

## 5. Gateway Profile Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `GATEWAY_PROFILE_NOT_FOUND` | 404 | NOT_FOUND | Gateway profile does not exist |
| `BELOW_MINIMUM_AMOUNT` | 400 | INVALID_ARGUMENT | Transaction below gateway minimum |
| `EXCEEDS_MAXIMUM_AMOUNT` | 400 | INVALID_ARGUMENT | Transaction exceeds gateway maximum |
| `DAILY_VOLUME_EXCEEDED` | 429 | RESOURCE_EXHAUSTED | Daily volume limit reached |
| `MONTHLY_VOLUME_EXCEEDED` | 429 | RESOURCE_EXHAUSTED | Monthly volume limit reached |
| `UNSUPPORTED_CARD_SCHEME` | 400 | INVALID_ARGUMENT | Card scheme not enabled for gateway |
| `UNSUPPORTED_CURRENCY` | 400 | INVALID_ARGUMENT | Currency not enabled for gateway |
| `GATEWAY_DISABLED` | 409 | FAILED_PRECONDITION | Gateway profile is disabled |
| `GATEWAY_MAINTENANCE` | 503 | UNAVAILABLE | Gateway in maintenance mode |
| `ALL_GATEWAYS_VOLUME_EXCEEDED` | 429 | RESOURCE_EXHAUSTED | All gateways hit daily limit |
| `ROTATION_STRATEGY_INVALID` | 400 | INVALID_ARGUMENT | Unknown rotation strategy |

---

## 6. Capability Flags

```rust
pub struct ConnectorCapabilities {
    pub supports_partial_capture: bool,
    pub supports_partial_refund: bool,
    pub supports_native_idempotency_key: bool,
    pub supports_webhook_settlement: bool,
    pub supports_realtime_status_check: bool,
    pub supports_fx_conversion: bool,               // Gap: acquirer can convert currency
    pub supported_card_schemes: Vec<CardScheme>,
    pub supported_currencies: Vec<CurrencyCode>,
    pub settlement_format: SettlementFormat,
    pub settlement_cycle: SettlementCycle,           // Gap: T+N settlement timing per acquirer
    pub cross_border_fee_bps: i32,                  // Gap: cross-border fee for routing cost calculation
}
```

## 3. Normalized Types

```rust
pub struct AuthorizeRequest {
    pub payment_method_token: String,
    pub amount: Money,
    pub idempotency_key: String,
    pub card_scheme: CardScheme,
    pub metadata: Option<serde_json::Value>,
}

pub struct AuthorizeResponse {
    pub status: AuthorizeStatus, // Approved | Declined | Requires3DS | PartialApproval
    pub acquirer_reference: Option<String>,
    pub decline_reason: Option<DeclineReason>,
    pub approved_amount: Option<Money>, // for partial auth
    pub three_ds_data: Option<ThreeDsData>, // Gap: 3DS passthrough — acquirer handles 3DS, we just pass through
    pub latency_ms: u32,
}

// Gap: 3DS passthrough — as an orchestrator, we do NOT implement 3DS.
// The acquirer/PSP handles 3DS challenge flow. We pass through three_ds_data
// so the merchant SDK can redirect the cardholder to the acquirer's 3DS page.
// After 3DS completion, the acquirer returns the final auth result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDsData {
    pub three_ds_version: String,       // "1.0" or "2.x"
    pub acs_url: Option<String>,        // acquirer's ACS URL for redirect
    pub pareq: Option<String>,          // PaReq token
    pub md: Option<String>,             // Merchant Data
    pub session_data: Option<String>,   // 3DS2 session data
}

// Gap: FX rate query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRateRequest {
    pub source_currency: CurrencyCode,
    pub target_currency: CurrencyCode,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRateResponse {
    pub rate: String,
    pub rate_minor_units: i64,     // rate * 10^6 for integer math
    pub converted_amount: Money,
    pub fee: Option<Money>,
    pub expires_at: DateTimeWithTimeZone,
}

// Gap: Settlement cycle per acquirer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementCycle {
    SameDay,            // T+0
    NextDay,            // T+1
    TwoDays,            // T+2
    ThreeDays,          // T+3
    Weekly,             // T+7
    Custom(u32),        // T+N
}

pub enum AuthorizeStatus {
    Approved,
    Declined,
    Requires3DS,
    PartialApproval,
}
```

## 4. Circuit Breaker

```rust
pub struct CircuitBreaker {
    state: CircuitState, // Closed | Open | HalfOpen
    error_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    open_duration: Duration, // default: 60s
    error_threshold: f64,    // default: 0.5 (50%)
    window: Duration,        // default: 30s
}
```

## 5. Per-Connector Retry Config

```rust
pub struct ConnectorRetryConfig {
    pub max_retries: u8,
    pub initial_backoff_ms: u32,
    pub backoff_multiplier: f32,
    pub max_backoff_ms: u32,
    pub jitter_percent: f32,
    pub retryable_error_codes: Vec<ConnectorError>,
}
```

## 6. Onboarding Schema & Credential Handling

### 6.1 Dynamic Onboarding Schema

Each connector declares its configuration fields. The dashboard renders forms dynamically.

```rust
pub struct OnboardingSchema {
    pub connector_id: String,
    pub fields: Vec<OnboardingField>,
}

pub struct OnboardingField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub label: String,
    pub validation_regex: Option<String>,
    pub help_text: Option<String>,
}

pub enum FieldType {
    String,
    Password,      // masked input
    Url,
    Integer,
    Select { options: Vec<SelectOption> },
}
```

**Example — Checkout.com Connector:**

```rust
impl AcquirerConnector for CheckoutComConnector {
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "checkout_com".into(),
            fields: vec![
                OnboardingField {
                    name: "api_key".into(),
                    field_type: FieldType::Password,
                    required: true,
                    label: "Secret Key".into(),
                    validation_regex: Some(r"^sk_(test|live)_[a-zA-Z0-9]+$".into()),
                    help_text: Some("Find in Dashboard > Settings > API Keys".into()),
                },
                OnboardingField {
                    name: "environment".into(),
                    field_type: FieldType::Select {
                        options: vec![
                            SelectOption { value: "sandbox".into(), label: "Sandbox (Test)".into() },
                            SelectOption { value: "production".into(), label: "Production (Live)".into() },
                        ],
                    },
                    required: true,
                    label: "Environment".into(),
                    validation_regex: None,
                    help_text: None,
                },
            ],
        }
    }
}
```

### 6.2 Credential Security

```rust
pub struct EncryptedConnectorConfig {
    pub encrypted_config: Vec<u8>, // envelope-encrypted JSON blob
    pub dek_wrapped: Vec<u8>,     // DEK wrapped by platform KEK
}

// CRED-001: All credentials encrypted at rest via envelope encryption
// CRED-002: Credentials never returned in plaintext via read API
// CRED-003: validate_credentials uses sandbox/status-check, never live-money call
```

**Credential Lifecycle:**

```rust
pub enum CredentialStatus {
    Active,
    Rotating,      // new key validated, old key still valid
    Expired,
    Revoked,
}

pub struct ConnectorCredential {
    pub link_id: Uuid,
    pub connector_id: String,
    pub encrypted_config: EncryptedConnectorConfig,
    pub status: CredentialStatus,
    pub previous_config: Option<EncryptedConnectorConfig>, // for dual-key rotation
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub rotated_at: Option<DateTime<Utc>>,
}
```

### 6.3 Credential Validation

```rust
pub async fn validate_and_connect(
    connector: &dyn AcquirerConnector,
    config: &ConnectorConfig,
) -> Result<MerchantAcquirerLink, PlatformError> {
    // 1. Validate credentials against acquirer's sandbox
    connector.validate_credentials(config).await?;

    // 2. Encrypt and store
    let encrypted = kms_client.encrypt(serde_json::to_vec(config)?, &encryption_context).await?;

    // 3. Create link in 'connected_untested' status
    let link = MerchantAcquirerLink::new(connector.connector_id(), encrypted);

    Ok(link)
}
```

---

## 7. Connector Registry

```rust
pub struct ConnectorRegistry {
    connectors: HashMap<String, Box<dyn AcquirerConnector>>,
}

impl ConnectorRegistry {
    pub fn register(&mut self, connector: Box<dyn AcquirerConnector>) {
        self.connectors.insert(connector.connector_id(), connector);
    }

    pub fn get(&self, connector_id: &str) -> Result<&dyn AcquirerConnector, PlatformError> {
        self.connectors.get(connector_id)
            .map(|c| c.as_ref())
            .ok_or_else(|| PlatformError::NotFound {
                resource: "connector".into(),
                id: Uuid::nil(),
            })
    }

    pub fn list_active(&self) -> Vec<&dyn AcquirerConnector> {
        self.connectors.values().map(|c| c.as_ref()).collect()
    }
}
```

---

## 8. Decline Code Normalization

Each connector maintains its own mapping table:

```rust
pub struct DeclineMappingTable {
    mappings: HashMap<String, DeclineReason>,
}

impl DeclineMappingTable {
    pub fn normalize(&self, raw_code: &str) -> DeclineReason {
        self.mappings.get(raw_code)
            .cloned()
            .unwrap_or_else(|| {
                // DECL-001: Unknown → log for improvement, default to UnknownError
                tracing::warn!(raw_code = %raw_code, "Unmapped decline code");
                DeclineReason::UnknownError(raw_code.to_string())
            })
    }
}
```

**Checkout.com Example Mapping:**

| Raw Code | Normalized | Retryable |
|---|---|---|
| ` insufficient_funds` | `InsufficientFunds` | Yes |
| `do_not_honor` | `DoNotHonor` | Configurable |
| `invalid_card_number` | `InvalidCard` | No |
| `expired_card` | `ExpiredCard` | No |
| `card_declined` | `SuspectedFraud` | No |
| `gateway_timeout` | `IssuerUnavailable` | Yes |
| `3ds_failed` | `ThreeDSecureFailed` | Configurable |
| `rate_limit_exceeded` | `RateLimitedByAcquirer` | Yes |

**Telr Example Mapping:**

| Raw Code | Normalized | Retryable |
|---|---|---|
| `110` (Insufficient funds) | `InsufficientFunds` | Yes |
| `103` (Invalid card) | `InvalidCard` | No |
| `104` (Expired card) | `ExpiredCard` | No |
| `109` (Do not honor) | `DoNotHonor` | Configurable |
| `115` (Issuer unavailable) | `IssuerUnavailable` | Yes |

---

## 9. Settlement Format Handling

### 9.1 Webhook Settlement

```rust
pub struct WebhookSettlementHandler {
    connector_id: String,
    signature_verifier: Box<dyn SignatureVerifier>,
}

impl WebhookSettlementHandler {
    pub async fn handle_inbound(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // 1. Verify signature
        self.signature_verifier.verify(headers, body)?;

        // 2. Parse webhook payload
        let event = self.parse_webhook(body)?;

        // 3. Normalize to RawSettlementRecord
        let records = event.settlement_records.into_iter()
            .map(|r| self.normalize_settlement(r))
            .collect();

        Ok(records)
    }
}
```

### 9.2 Polling API Settlement

```rust
pub struct PollingSettlementHandler {
    connector: Box<dyn AcquirerConnector>,
    last_poll_cursor: Option<String>,
}

impl PollingSettlementHandler {
    pub async fn poll(&self) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        let records = self.connector.poll_settlement(PollSettlementRequest {
            since: self.last_poll_cursor.clone(),
            limit: 1000,
        }).await?;

        Ok(records)
    }
}
```

### 9.3 SFTP Settlement

```rust
pub struct SftpSettlementHandler {
    sftp_client: SftpClient,
    connector_id: String,
}

impl SftpSettlementHandler {
    pub async fn watch_and_ingest(&self) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // 1. Connect with host key pinning
        let session = self.sftp_client.connect(&self.sftp_config).await?;

        // 2. List new files (since last successful download)
        let files = session.list_dir("/settlement/").await?;

        let mut all_records = vec![];
        for file in files {
            // 3. Download and verify checksum
            let content = session.download(&file.path).await?;
            let checksum = Sha256::digest(&content);

            // 4. Parse CSV/XML/PDF format
            let records = self.parse_settlement_file(&content, &file.format)?;

            // 5. Log to audit trail
            self.audit_log(SettlementFileIngested {
                filename: file.name,
                checksum: hex::encode(checksum),
                record_count: records.len(),
            }).await?;

            all_records.extend(records);
        }

        Ok(all_records)
    }
}
```

### 9.4 Scanned Document Settlement

```rust
pub struct ScannedSettlementHandler {
    document_service: DocumentServiceClient,
    ai_gateway: AiGatewayClient,
}

impl ScannedSettlementHandler {
    pub async fn extract_from_pdf(&self, pdf_bytes: &[u8]) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // 1. Upload to document-service
        let doc = self.document_service.upload(pdf_bytes, "settlement_advice").await?;

        // 2. Trigger OCR via ai-gateway (Qwen3-VL 8B)
        let extraction = self.ai_gateway.extract_settlement(doc.id).await?;

        // 3. Parse extracted structured data into RawSettlementRecords
        let records = self.parse_extracted_data(extraction)?;

        Ok(records)
    }
}
```

---

## 10. Bulkhead Isolation

```rust
pub struct AdapterBulkhead {
    client: reqwest::Client,
    pool_size: usize,
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl AdapterBulkhead {
    pub fn new(config: &BulkheadConfig) -> Self {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(config.pool_size)
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            pool_size: config.pool_size,
            connect_timeout: config.connect_timeout,
            request_timeout: config.request_timeout,
        }
    }
}

// Each adapter has its own BulkheadConfig:
// - authorize/capture: pool_size=10, timeout=10s
// - settlement polling: pool_size=2, timeout=30s
// - webhook delivery: pool_size=5, timeout=10s
```

---

## 11. Conformance Test Suite

```rust
#[cfg(test)]
mod conformance_tests {
    /// Shared conformance suite — every connector must pass 100%
    /// Run against connector's sandbox environment

    #[tokio::test]
    async fn test_authorize_approved() {
        let connector = get_sandbox_connector();
        let result = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        assert_eq!(result.status, AuthorizeStatus::Approved);
        assert!(result.acquirer_reference.is_some());
    }

    #[tokio::test]
    async fn test_authorize_declined_insufficient_funds() {
        let connector = get_sandbox_connector();
        let result = connector.authorize(AuthorizeRequest::test_declined("insufficient_funds")).await.unwrap();
        assert_eq!(result.status, AuthorizeStatus::Declined);
        assert_eq!(result.decline_reason, Some(DeclineReason::InsufficientFunds));
    }

    #[tokio::test]
    async fn test_capture_full() {
        let connector = get_sandbox_connector();
        let auth = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        let result = connector.capture(CaptureRequest {
            acquirer_reference: auth.acquirer_reference.unwrap(),
            amount: Money { amount_minor_units: 10000, currency: CurrencyCode::new("AED").unwrap() },
        }).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_void() {
        let connector = get_sandbox_connector();
        let auth = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        let result = connector.void(VoidRequest {
            acquirer_reference: auth.acquirer_reference.unwrap(),
        }).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_refund_full() {
        let connector = get_sandbox_connector();
        let auth = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        connector.capture(CaptureRequest { ... }).await.unwrap();
        let result = connector.refund(RefundRequest {
            acquirer_reference: auth.acquirer_reference.unwrap(),
            amount: Money { amount_minor_units: 10000, currency: CurrencyCode::new("AED").unwrap() },
        }).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let connector = get_sandbox_connector().with_slow_network(Duration::from_secs(20));
        let result = connector.authorize(AuthorizeRequest::test_approved()).await;
        assert!(matches!(result, Err(ConnectorError::Timeout)));
    }

    #[tokio::test]
    async fn test_webhook_signature_valid() {
        let connector = get_sandbox_connector();
        let (headers, body) = connector.create_test_webhook();
        assert!(connector.verify_webhook_signature(&headers, &body).is_ok());
    }

    #[tokio::test]
    async fn test_webhook_signature_tampered() {
        let connector = get_sandbox_connector();
        let (_, body) = connector.create_test_webhook();
        let mut headers = HeaderMap::new();
        headers.insert("X-Signature", "tampered".parse().unwrap());
        assert!(connector.verify_webhook_signature(&headers, &body).is_err());
    }

    #[tokio::test]
    async fn test_idempotency_native() {
        if !connector.supports_native_idempotency() { return; }
        let connector = get_sandbox_connector();
        let idem_key = uuid::Uuid::now_v7().to_string();
        let r1 = connector.authorize(AuthorizeRequest::test_with_idem(&idem_key)).await.unwrap();
        let r2 = connector.authorize(AuthorizeRequest::test_with_idem(&idem_key)).await.unwrap();
        assert_eq!(r1.acquirer_reference, r2.acquirer_reference);
    }

    #[tokio::test]
    async fn test_idempotency_status_check() {
        if connector.supports_native_idempotency() { return; }
        let connector = get_sandbox_connector();
        let idem_key = uuid::Uuid::now_v7().to_string();
        let r1 = connector.authorize(AuthorizeRequest::test_with_idem(&idem_key)).await.unwrap();
        let r2 = connector.status_check(StatusCheckRequest {
            acquirer_reference: r1.acquirer_reference.unwrap(),
        }).await.unwrap();
        assert!(r2.status != AuthorizeStatus::Unknown);
    }
}
```

---

## 12. Card Scheme Compliance Monitoring

```rust
pub struct SchemeComplianceMonitor {
    analytics_db: ClickHouseClient,
    notification_service: NotificationClient,
}

impl SchemeComplianceMonitor {
    pub async fn check_compliance(&self) -> Result<Vec<ComplianceAlert>, PlatformError> {
        let mut alerts = vec![];

        // Visa chargeback ratio (30-day rolling)
        let visa_cb = self.analytics_db.query_chargeback_ratio("visa", 30).await?;
        if visa_cb > 0.009 {
            alerts.push(ComplianceAlert {
                scheme: "Visa".into(),
                metric: "chargeback_ratio".into(),
                current: visa_cb,
                threshold: 0.01,
                severity: if visa_cb > 0.01 { "critical" } else { "warning" },
            });
        }

        // Mastercard chargeback ratio (30-day rolling)
        let mc_cb = self.analytics_db.query_chargeback_ratio("mastercard", 30).await?;
        if mc_cb > 0.0135 {
            alerts.push(ComplianceAlert {
                scheme: "Mastercard".into(),
                metric: "chargeback_ratio".into(),
                current: mc_cb,
                threshold: 0.015,
                severity: if mc_cb > 0.015 { "critical" } else { "warning" },
            });
        }

        // Send alerts
        for alert in &alerts {
            self.notification_service.send_compliance_alert(alert).await?;
        }

        Ok(alerts)
    }
}
```

---

## 13. Connector-Specific Implementation Examples

### Network International (UAE Regional Acquirer)

```rust
pub struct NetworkInternationalConnector {
    api_key: String,
    merchant_id: String,
    environment: String, // 'sandbox' | 'production'
    base_url: String,
    bulkhead: AdapterBulkhead,
    circuit_breaker: CircuitBreaker,
    decline_table: DeclineMappingTable,
}

#[async_trait]
impl AcquirerConnector for NetworkInternationalConnector {
    fn connector_id(&self) -> ConnectorId { "network_international".into() }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: true,
            supports_partial_refund: true,
            supports_native_idempotency_key: false, // use status-check
            supports_webhook_settlement: true,
            supports_realtime_status_check: true,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            supported_currencies: vec![CurrencyCode::new("AED").unwrap()],
            settlement_format: SettlementFormat::Webhook,
        }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        // 1. Build NI-specific request
        let ni_req = self.build_authorize_request(&req)?;

        // 2. Call NI API via bulkhead-isolated client
        let ni_resp = self.bulkhead.post(&format!("{}/api/v1/authorize", self.base_url), &ni_req).await?;

        // 3. Normalize response
        let status = self.classify_status(&ni_resp);
        let decline = ni_resp.response_code.as_ref()
            .map(|code| self.decline_table.normalize(code));

        Ok(AuthorizeResponse {
            status,
            acquirer_reference: ni_resp.transaction_id,
            decline_reason: decline,
            approved_amount: ni_resp.approved_amount.map(|a| Money { amount_minor_units: a, currency: req.amount.currency }),
            three_ds_data: ni_resp.three_ds,
            latency_ms: ni_resp.latency_ms,
        })
    }

    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "network_international".into(),
            fields: vec![
                OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "API Key".into(), validation_regex: None, help_text: None },
                OnboardingField { name: "merchant_id".into(), field_type: FieldType::String, required: true, label: "Merchant ID".into(), validation_regex: None, help_text: None },
                OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![
                    SelectOption { value: "sandbox".into(), label: "Sandbox".into() },
                    SelectOption { value: "production".into(), label: "Production".into() },
                ]}, required: true, label: "Environment".into(), validation_regex: None, help_text: None },
            ],
        }
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<(), ConnectorError> {
        // CRED-003: sandbox/status-check call only, never live-money
        let resp = self.bulkhead.get(&format!("{}/api/v1/merchant/status", self.base_url)).await?;
        if resp.status_code != 200 {
            return Err(ConnectorError::AuthenticationFailed);
        }
        Ok(())
    }
}
```

### Checkout.com (International PSP)

```rust
pub struct CheckoutComConnector {
    secret_key: String,
    environment: String,
    base_url: String,
    bulkhead: AdapterBulkhead,
    circuit_breaker: CircuitBreaker,
    decline_table: DeclineMappingTable,
}

#[async_trait]
impl AcquirerConnector for CheckoutComConnector {
    fn connector_id(&self) -> ConnectorId { "checkout_com".into() }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: true,
            supports_partial_refund: true,
            supports_native_idempotency_key: true, // Checkout.com has idempotency keys
            supports_webhook_settlement: true,
            supports_realtime_status_check: true,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex],
            supported_currencies: vec![
                CurrencyCode::new("AED").unwrap(),
                CurrencyCode::new("USD").unwrap(),
                CurrencyCode::new("EUR").unwrap(),
                CurrencyCode::new("GBP").unwrap(),
            ],
            settlement_format: SettlementFormat::Webhook,
        }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let cc_req = self.build_authorize_request(&req)?;
        let cc_resp = self.bulkhead.post(&format!("{}/payments", self.base_url), &cc_req).await?;
        Ok(self.normalize_authorize_response(&cc_resp)?)
    }

    // Checkout.com uses native idempotency keys
    // No status-check-before-retry needed for this connector
}
```

### Telr (Regional PSP)

```rust
pub struct TelrConnector {
    store_id: String,
    api_key: String,
    environment: String,
    base_url: String,
    bulkhead: AdapterBulkhead,
    circuit_breaker: CircuitBreaker,
    decline_table: DeclineMappingTable,
}

#[async_trait]
impl AcquirerConnector for TelrConnector {
    fn connector_id(&self) -> ConnectorId { "telr".into() }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: false, // Telr doesn't support partial capture
            supports_partial_refund: true,
            supports_native_idempotency_key: false,
            supports_webhook_settlement: false, // Polling API only
            supports_realtime_status_check: true,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            supported_currencies: vec![CurrencyCode::new("AED").unwrap(), CurrencyCode::new("USD").unwrap()],
            settlement_format: SettlementFormat::PollingApi,
        }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let telr_req = self.build_authorize_request(&req)?;
        let telr_resp = self.bulkhead.post(&format!("{}/api/v2/order/preauth", self.base_url), &telr_req).await?;
        Ok(self.normalize_authorize_response(&telr_resp)?)
    }

    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // Telr uses polling API for settlement
        let resp = self.bulkhead.get(&format!(
            "{}/api/v2/order/list?since={}", self.base_url, req.since.unwrap_or_default()
        )).await?;
        self.parse_settlement_list(&resp)
    }
}
```

---

## 14. TDD Tests (Extended)

```rust
#[tokio::test]
async fn test_authorize_approved() {
    let connector = MockConnector::new().with_response(AuthorizeResponse::approved());
    let result = connector.authorize(AuthorizeRequest { ... }).await.unwrap();
    assert_eq!(result.status, AuthorizeStatus::Approved);
}

#[tokio::test]
async fn test_authorize_declined() {
    let connector = MockConnector::new().with_response(AuthorizeResponse::declined(DeclineReason::InsufficientFunds));
    let result = connector.authorize(AuthorizeRequest { ... }).await.unwrap();
    assert_eq!(result.status, AuthorizeStatus::Declined);
    assert_eq!(result.decline_reason, Some(DeclineReason::InsufficientFunds));
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_high_error_rate() {
    let connector = MockConnector::new().with_failing_rate(0.6); // 60% failure
    // Send 10 requests — circuit should open after error threshold exceeded
    for _ in 0..10 {
        connector.authorize(AuthorizeRequest { ... }).await.ok();
    }
    assert!(connector.circuit_breaker().is_open());
}

#[tokio::test]
async fn test_circuit_breaker_half_open_after_timeout() {
    let connector = MockConnector::new().with_failing_rate(1.0);
    // Trigger circuit open
    for _ in 0..10 {
        connector.authorize(AuthorizeRequest { ... }).await.ok();
    }
    // Wait for open duration
    tokio::time::sleep(Duration::from_secs(61)).await;
    assert!(connector.circuit_breaker().is_half_open());
}

#[tokio::test]
async fn test_webhook_signature_verification() {
    let connector = MockConnector::new();
    let headers = HeaderMap::new();
    let body = b"test payload";
    let valid_sig = connector.compute_signature(body);
    headers.insert("X-Signature", valid_sig.parse().unwrap());
    assert!(connector.verify_webhook_signature(&headers, body).is_ok());
}

#[tokio::test]
async fn test_webhook_signature_tampered_rejected() {
    let connector = MockConnector::new();
    let headers = HeaderMap::new();
    let body = b"test payload";
    headers.insert("X-Signature", "tampered".parse().unwrap());
    assert!(connector.verify_webhook_signature(&headers, body).is_err());
}
```


