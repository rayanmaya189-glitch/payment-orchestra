# 23 — Shared Types Addendum (New Types from Gap Analysis)

**Purpose**: New shared types identified by the comprehensive gap analysis. These should be merged into `00-shared-types.md` during implementation.

---

## S1. 3D Secure Types (New)

```rust
/// 3D Secure authentication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeDsStatus {
    NotRequired,            // 3DS not required (exemption applied)
    Frictionless,           // 3DS authenticated without challenge
    ChallengeRequired,      // 3DS requires cardholder challenge
    AuthenticationFailed,   // 3DS authentication failed
    AuthenticationUnavailable, // 3DS server unavailable
}

/// 3D Secure authentication data (passed to/from merchant SDK)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDsData {
    pub three_ds_version: String,      // "1.0.2" or "2.1.0" or "2.2.0"
    pub status: ThreeDsStatus,
    pub acs_url: Option<String>,       // ACS challenge URL
    pub pareq: Option<String>,         // PaReq (3DS 1.0)
    pub md: Option<String>,            // Merchant Data (3DS 1.0)
    pub cres: Option<String>,          // Challenge Response (3DS 2.x)
    pub session_data: Option<String>,  // 3DS 2.x session
    pub eci: Option<String>,           // Electronic Commerce Indicator
    pub cavv: Option<String>,          // Cardholder Authentication Verification Value
    pub ds_transaction_id: Option<String>, // Directory Server transaction ID
    pub three_ds_server_trans_id: Option<String>, // 3DS Server transaction ID
}

/// 3DS enrollment check request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check3dsRequest {
    pub card_bin: String,
    pub card_last_four: String,
    pub card_expiry_month: u8,
    pub card_expiry_year: u16,
    pub amount: Money,
    pub merchant_name: String,
    pub merchant_url: String,
    pub browser_info: Option<BrowserInfo>,
    pub recurring: bool,               // MIT (merchant-initiated)
    pub exemption_requested: Option<ThreeDsExemption>,
}

/// 3DS exemption types (PSD2 SCA exemptions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeDsExemption {
    LowValue,           // < 30 EUR (configurable)
    LowRisk,            // Merchant's low-risk transaction
    TrustedMerchant,    // Cardholder's whitelisted merchant
    SecureCorporate,    // Corporate card with secure payment
    Delegated,          // Delegated authentication
    TransactionRiskAnalysis, // TRA exemption
}

/// Browser info for 3DS 2.x
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub user_agent: String,
    pub accept_header: String,
    pub language: String,
    pub color_depth: u32,
    pub screen_height: u32,
    pub screen_width: u32,
    pub timezone_offset: i32,
    pub java_enabled: bool,
    pub javascript_enabled: bool,
}
```

---

## S2. Circuit Breaker Types (New)

```rust
/// Circuit breaker state for acquirer connections
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BreakerState {
    Closed,     // Normal operation — requests pass through
    Open,       // Failing — requests are rejected immediately
    HalfOpen,   // Testing recovery — limited requests allowed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub error_threshold_percent: f64,    // e.g., 50% = open at 50% failure rate
    pub window_seconds: u32,             // rolling window: 30s default
    pub open_duration_seconds: u32,      // stay open: 60s default
    pub min_sample_size: u32,            // min requests before evaluating: 10 default
    pub half_open_max_requests: u32,     // requests allowed in half-open: 3 default
    pub consecutive_successes_to_close: u32, // successes to close: 5 default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    pub link_id: Uuid,
    pub state: BreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_at: Option<DateTimeWithTimeZone>,
    pub opened_at: Option<DateTimeWithTimeZone>,
    pub half_open_attempts: u32,
    pub consecutive_successes: u32,
}

impl CircuitBreakerState {
    /// After half-open, if enough consecutive successes → close
    pub fn should_close(&self, config: &CircuitBreakerConfig) -> bool {
        self.state == BreakerState::HalfOpen
            && self.consecutive_successes >= config.consecutive_successes_to_close
    }

    /// After open duration + ttl → transition to half-open
    pub fn should_half_open(&self, config: &CircuitBreakerConfig) -> bool {
        if self.state != BreakerState::Open { return false; }
        self.opened_at.map_or(false, |t| {
            Utc::now().signed_duration_since(t).num_seconds()
                >= config.open_duration_seconds as i64
        })
    }
}
```

---

## S3. BYOK / Credential Management Types (New)

```rust
/// BYOK link environment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkEnvironment {
    Sandbox,
    Production,
}

/// BYOK link status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkStatus {
    Testing,              // Credentials saved but not validated
    Active,               // Validated and operational
    Disabled,             // Manually disabled by merchant
    CredentialsExpired,   // Credentials have expired
    Revoked,              // Revoked by platform (security)
}

/// BYOK link health
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkHealth {
    Healthy,
    Degraded,    // Intermittent failures
    Unreachable, // Currently not responding
    Unknown,     // Not yet tested
}

/// Encrypted credentials container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCredentials {
    pub key_identifier: String,      // KMS key version identifier
    pub encrypted_payload: Vec<u8>,  // AES-256-GCM encrypted JSON blob
    pub nonce: Vec<u8>,              // 12-byte GCM nonce
    pub wrapped_dek: Vec<u8>,        // DEK wrapped by KEK
    pub encryption_version: u32,     // For key rotation tracking
}

/// Raw connector credentials (from merchant form, never stored)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawConnectorCredentials {
    pub fields: HashMap<String, String>,
}

/// Connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub latency_ms: u32,
    pub merchant_name: Option<String>,    // Verified merchant name from gateway
    pub permissions: Vec<String>,         // authorize, capture, refund, void
    pub error_message: Option<String>,
}

/// Credential validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialValidationResult {
    pub valid: bool,
    pub merchant_name: Option<String>,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub error_message: Option<String>,
}

/// Dynamic onboarding schema (from connector-gateway to merchant form)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingSchema {
    pub connector_id: String,
    pub display_name: String,
    pub description: String,
    pub fields: Vec<OnboardingField>,
    pub test_card_numbers: Vec<TestCardNumber>,
    pub supported_environments: Vec<String>,
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingField {
    pub name: String,
    pub field_type: CredentialFieldType,
    pub required: bool,
    pub label: String,
    pub placeholder: Option<String>,
    pub validation: Option<FieldValidation>,
    pub help_text: Option<String>,
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialFieldType {
    String,
    Password,
    Url,
    Integer,
    Select { options: Vec<SelectOption> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValidation {
    pub regex: Option<String>,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCardNumber {
    pub brand: String,
    pub number: String,
    pub card_type: TestCardType,  // success, decline, insufficient_funds, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestCardType {
    Success,
    Decline,
    InsufficientFunds,
    DoNotHonor,
    Requires3DS,
    LostCard,
    StolenCard,
}
```

---

## S4. Network Token Types (New)

```rust
/// Network token (Visa Token Service / Mastercard MDES)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenInfo {
    pub token_id: Uuid,
    pub network: CardScheme,
    pub token_reference: String,       // network token reference (e.g., "vts_abc123")
    pub token_type: NetworkTokenType,  // Visa Token, Mastercard Digital Secure Remote Payment
    pub device_bin: Option<String>,    // token BIN
    pub device_last_four: String,      // token last 4 digits
    pub card_bin: String,              // actual card BIN
    pub card_last_four: String,        // actual card last 4 digits
    pub card_expiry_month: u8,
    pub card_expiry_year: u16,
    pub token_status: TokenStatus,
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: Option<DateTimeWithTimeZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkTokenType {
    VisaToken,              // Visa Token Service
    MastercardDigital,      // Mastercard Digital Enablement Service
    AmexToken,              // Amex Token Service
}

/// Account Updater result (Visa Account Updater / Mastercard ABU)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdateResult {
    pub token_id: Uuid,
    pub updated: bool,
    pub new_expiry_month: Option<u8>,
    pub new_expiry_year: Option<u16>,
    pub new_last_four: Option<String>,
    pub card_status: AccountUpdaterCardStatus, // same, changed, closed, contact_customer
    pub source: String, // "visa" | "mastercard"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountUpdaterCardStatus {
    Same,              // No changes
    Changed,           // Card reissued with new details
    Closed,            // Account closed
    ContactCustomer,   // Contact cardholder for new details
}
```

---

## S5. A/B Testing & Canary Routing Types (New)

```rust
/// Canary routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    pub acquirer_link_id: Uuid,
    pub traffic_percentage: f64,             // 0.01 = 1% of traffic
    pub conditions: Vec<RoutingCondition>,   // only route matching transactions
    pub max_amount: Option<Money>,
    pub min_sample_size: u32,               // min transactions for statistical significance
    pub duration_days: u32,
    pub auto_promote: bool,                  // auto-promote if success rate > threshold
    pub promotion_threshold: Option<f64>,    // success rate threshold for auto-promotion
    pub created_at: DateTimeWithTimeZone,
}

/// A/B test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbTestResult {
    pub canary_link_id: Uuid,
    pub control_link_id: Uuid,
    pub canary_sample_size: u32,
    pub control_sample_size: u32,
    pub canary_success_rate: f64,
    pub control_success_rate: f64,
    pub canary_avg_latency_ms: f64,
    pub control_avg_latency_ms: f64,
    pub canary_avg_fee: Option<Money>,
    pub control_avg_fee: Option<Money>,
    pub statistically_significant: bool,
    pub recommendation: Option<String>,     // "promote" | "reject" | "continue"
}
```

---

## S6. Enhanced Payment Status (with 3DS states)

```rust
/// Enhanced PaymentStatus with 3DS support
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Created,
    Authorizing,
    ThreeDsRequired,          // NEW: awaiting 3DS authentication
    ThreeDsAuthenticating,    // NEW: 3DS challenge in progress
    ThreeDsFailed,            // NEW: 3DS authentication failed
    Authorized,
    Capturing,
    Captured,
    PartiallyCaptured,
    Voided,
    AuthorizationExpired,
    Failed,
    FailedAllRoutes,
    Refunding,
    RefundPending,            // NEW: refund queued (acquirer offline)
    Refunded,
    PartiallyRefunded,
}

impl PaymentStatus {
    /// Check if a command is valid from this state
    pub fn can_transition(&self, command: &PaymentCommand) -> bool {
        match (self, command) {
            (Self::Created, PaymentCommand::Authorize) => true,
            (Self::Authorizing, PaymentCommand::Authorize) => true,
            // NEW: 3DS transitions
            (Self::Authorizing, PaymentCommand::Initiate3DS) => true,
            (Self::ThreeDsRequired, PaymentCommand::Authenticate3DS) => true,
            (Self::ThreeDsAuthenticating, PaymentCommand::Complete3DS) => true,
            (Self::ThreeDsFailed, PaymentCommand::Authorize) => true, // retry without 3DS
            // Existing transitions
            (Self::Authorized, PaymentCommand::Capture) => true,
            (Self::Authorized, PaymentCommand::Void) => true,
            (Self::PartiallyCaptured, PaymentCommand::Capture) => true,
            (Self::Captured | Self::PartiallyCaptured, PaymentCommand::Refund) => true,
            (Self::RefundPending, PaymentCommand::Refund) => true, // retry refund
            _ => false,
        }
    }
}

/// NEW: Enhanced DeclineReason with more specific codes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclineReason {
    InsufficientFunds,
    DoNotHonor,
    InvalidCard,
    ExpiredCard,
    SuspectedFraud,
    LostCard,                    // NEW
    StolenCard,                  // NEW
    PickUpCard,                  // NEW: card should be retained
    IssuerUnavailable,
    ThreeDSecureFailed,
    ThreeDSecureRequired,        // NEW: acquirer requires 3DS
    RateLimitedByAcquirer,
    PartialAuthorizationRejected,
    HighRiskDeclined,            // NEW: risk service rejected
    InvalidCvv,                  // NEW
    InvalidExpiry,               // NEW
    RestrictedCard,              // NEW: card type not allowed
    ExceedsWithdrawalLimit,      // NEW
    ViolationOfLaw,              // NEW: AML/KYC restriction
    DuplicateTransaction,        // NEW: detected by acquirer
    ReenterTransaction,          // NEW: acquirer-specific retryable
    UnknownError(String),
}
```

---

## S7. Enhanced CardScheme with Network Token Support

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CardScheme {
    Visa,
    Mastercard,
    Amex,
    Mada,
    UnionPay,
    Jcb,
    Discover,
    Diners,
    CartesBancaires,
    Other(String),
}

impl CardScheme {
    /// Does this scheme support network tokens?
    pub fn supports_network_tokens(&self) -> bool {
        matches!(self, Self::Visa | Self::Mastercard | Self::Amex)
    }

    /// Does this scheme have a 3DS requirement?
    pub fn requires_3ds(&self, region: &str) -> bool {
        matches!(
            (self, region),
            (Self::Visa | Self::Mastercard | Self::Amex, "europe")
                | (Self::Mada, "saudi_arabia")
        )
    }
}
```

---

## S8. Statement Descriptor Management (New)

```rust
/// Statement descriptor controls what appears on the cardholder's statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementDescriptor {
    /// Static descriptor (e.g., "ACME CORP*")
    pub static_descriptor: String,
    /// Dynamic descriptor prefix (e.g., "ACME CORP*ORDER-123")
    pub dynamic_prefix: Option<String>,
    /// City for descriptor (shown on statement)
    pub city: Option<String>,
    /// Phone number for descriptor
    pub phone: Option<String>,
    /// URL for descriptor
    pub url: Option<String>,
}

impl StatementDescriptor {
    /// Maximum length for Visa (25 chars including * and spaces)
    pub const VISA_MAX_LENGTH: usize = 25;
    /// Maximum length for Mastercard (22 chars)
    pub const MASTERCARD_MAX_LENGTH: usize = 22;

    pub fn validate(&self, card_scheme: &CardScheme) -> Result<(), ValidationError> {
        let max = match card_scheme {
            CardScheme::Visa => Self::VISA_MAX_LENGTH,
            CardScheme::Mastercard => Self::MASTERCARD_MAX_LENGTH,
            _ => 25,
        };
        if self.static_descriptor.len() > max {
            return Err(ValidationError::MissingField("Statement descriptor exceeds length limit".into()));
        }
        Ok(())
    }
}
```

---

## S9. Enhanced Routing Types (Multi-Dimensional)

```rust
/// Enhanced routing condition with more dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub card_scheme: Option<CardScheme>,
    pub currency: Option<CurrencyCode>,
    pub min_amount: Option<Money>,
    pub max_amount: Option<Money>,
    // NEW dimensions
    pub card_bin_prefix: Option<String>,          // e.g., "4" for Visa, "5" for MC, "34|37" for Amex
    pub card_country: Option<String>,             // card issuing country
    pub billing_country: Option<String>,          // billing address country
    pub payment_method_type: Option<PaymentMethodType>, // card, wallet, bank_account
    pub recurring: Option<bool>,                  // CIT vs MIT
    pub risk_score_min: Option<f64>,              // only route if risk >= this
    pub risk_score_max: Option<f64>,              // only route if risk <= this
}

/// Enhanced failover config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub retryable_decline_codes: Vec<DeclineReason>,
    pub max_hops: u8,                // platform ceiling: 3
    pub latency_budget_ms: u32,      // total checkout latency budget
    pub retry_unknown_as_fallback: bool,
    // NEW
    pub circuit_breaker_aware: bool, // skip acquirers with open circuit breaker
    pub timeout_ms_per_hop: u32,     // max time per single hop
    pub enable_optimistic_racing: bool, // fire first 2 acquirers in parallel, take first success
}

/// Rotation strategy (enhanced)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    Priority,
    RoundRobin,
    WeightedRoundRobin { weights: Vec<(Uuid, u32)> },
    CostBased,
    SuccessRateBased,
    VolumeCapped,
    /// NEW: Lowest authorization cost (fee ÷ success_rate)
    CostAdjustedForSuccess,
    /// NEW: Fastest response time
    LowestLatency,
    /// NEW: Random (for A/B testing split)
    Random { seed: Option<u64> },
}
```

---

## S10. Webhook Delivery Enhancement Types

```rust
/// Enhanced webhook event types (more events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebhookEventType {
    // Payment lifecycle
    PaymentCreated,
    PaymentAuthorized,
    PaymentCaptured,
    PaymentFailed,
    PaymentRefunded,
    PaymentVoided,
    PaymentPartiallyCaptured,
    PaymentPartiallyRefunded,
    // NEW: 3DS events
    PaymentThreeDsRequired,
    PaymentThreeDsCompleted,
    PaymentThreeDsFailed,
    // Settlement
    SettlementMatched,
    SettlementUnmatched,
    // Disputes
    ChargebackReceived,
    ChargebackResolved,
    ChargebackRepresentmentDue,      // NEW
    // Risk
    HighRiskTransactionDetected,     // NEW
    // Subscriptions
    SubscriptionCreated,             // NEW
    SubscriptionRenewed,             // NEW
    SubscriptionPaused,              // NEW
    SubscriptionCancelled,           // NEW
    SubscriptionPaymentFailed,       // NEW
}

/// Webhook delivery retry policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRetryPolicy {
    pub max_attempts: u32,                     // default: 8
    pub initial_backoff_ms: u32,               // default: 1000 (1s)
    pub backoff_multiplier: f64,               // default: 3.0
    pub max_backoff_ms: u32,                   // default: 28800000 (8 hours)
    pub jitter_percent: f64,                   // default: 0.1 (10%)
}

impl WebhookRetryPolicy {
    pub fn default_retry_schedule(&self) -> Vec<u32> {
        let mut schedule = vec![];
        let mut delay = self.initial_backoff_ms;
        for _ in 0..self.max_attempts {
            let jitter = (delay as f64 * self.jitter_percent * rand::random::<f64>()) as u32;
            schedule.push(delay + jitter);
            delay = ((delay as f64) * self.backoff_multiplier) as u32;
            delay = delay.min(self.max_backoff_ms);
        }
        schedule
    }
}
```
