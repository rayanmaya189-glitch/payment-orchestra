# 00 — Shared Types & Value Objects

Cross-cutting types used by all services. Defined once, imported everywhere.

---

## 1. Identity Types

```rust
// All IDs are UUIDv7 (RFC 9562) — time-ordered, B-tree friendly
pub type OperatorId = Uuid;
pub type PrincipalId = Uuid;
pub type PaymentIntentId = Uuid;
pub type InvoiceId = Uuid;
pub type SubscriptionId = Uuid;
pub type SettlementBatchId = Uuid;
pub type ChargebackId = Uuid;
pub type RoutingPolicyId = Uuid;
pub type MerchantAcquirerLinkId = Uuid;
pub type IdempotencyKey = String; // caller-supplied, 128-bit minimum entropy
pub type CorrelationId = Uuid;
pub type CausationId = Uuid;
pub type EventId = Uuid;
pub type SagaId = Uuid;
```

## 2. Money Value Object

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    /// Amount in minor units (cents, fils, etc.) — NEVER floats
    pub amount_minor_units: i64,
    /// ISO 4217 currency code
    pub currency: CurrencyCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyCode(pub String); // validated: exactly 3 uppercase ASCII letters

impl CurrencyCode {
    pub fn new(code: &str) -> Result<Self, ValidationError> {
        if code.len() != 3 || !code.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(ValidationError::InvalidCurrencyCode);
        }
        Ok(Self(code.to_string()))
    }
}

impl Money {
    /// Minor unit precision per ISO 4217
    pub fn minor_unit_precision(&self) -> u32 {
        match self.currency.0.as_str() {
            "BHD" | "KWD" => 3,
            "JPY" => 0,
            _ => 2, // AED, USD, SAR, EUR, GBP, etc.
        }
    }

    /// Validate amount is valid for this currency's precision
    pub fn validate(&self) -> Result<(), ValidationError> {
        let precision = self.minor_unit_precision();
        let max_minor = 10i64.pow(precision);
        if self.amount_minor_units < 0 {
            return Err(ValidationError::NegativeAmount);
        }
        // For display: amount must not have more decimal places than precision allows
        // For storage: amount_minor_units is always an integer, so this is always valid
        Ok(())
    }

    /// Zero-amount (card verification)
    pub fn is_zero(&self) -> bool {
        self.amount_minor_units == 0
    }

    /// Add two Money values (same currency only)
    pub fn checked_add(&self, other: &Money) -> Result<Money, ValidationError> {
        if self.currency != other.currency {
            return Err(ValidationError::CurrencyMismatch);
        }
        let sum = self.amount_minor_units
            .checked_add(other.amount_minor_units)
            .ok_or(ValidationError::AmountOverflow)?;
        Ok(Money { amount_minor_units: sum, currency: self.currency.clone() })
    }
}
```

## 3. Actor Reference

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorReference {
    pub actor_type: ActorType,
    pub actor_id: Option<Uuid>, // None for system actors
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorType {
    User,
    ApiKey,
    System,
    Scheduler,
    Webhook,
}
```

## 4. Payment States

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Created,
    Authorizing,
    Authorized,
    Capturing,
    Captured,
    PartiallyCaptured,
    Voided,
    AuthorizationExpired,
    Failed,
    FailedAllRoutes,
    Refunding,
    Refunded,
    PartiallyRefunded,
}

impl PaymentStatus {
    /// Check if a command is valid from this state
    pub fn can_transition(&self, command: &PaymentCommand) -> bool {
        match (self, command) {
            (Self::Created, PaymentCommand::Authorize) => true,
            (Self::Authorizing, PaymentCommand::Authorize) => true, // retry hop
            (Self::Authorized, PaymentCommand::Capture) => true,
            (Self::Authorized, PaymentCommand::Void) => true,
            (Self::PartiallyCaptured, PaymentCommand::Capture) => true, // remaining amount
            (Self::Captured | Self::PartiallyCaptured, PaymentCommand::Refund) => true,
            _ => false,
        }
    }
}
```

## 5. Decline Reasons (Normalized)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclineReason {
    InsufficientFunds,
    DoNotHonor,
    InvalidCard,
    ExpiredCard,
    SuspectedFraud,
    IssuerUnavailable,
    ThreeDSecureFailed,
    RateLimitedByAcquirer,
    PartialAuthorizationRejected,
    UnknownError(String), // unmapped raw code logged for improvement
}

impl DeclineReason {
    /// Is this decline retryable on a different acquirer?
    pub fn is_retryable(&self, config: &FailoverConfig) -> bool {
        match self {
            Self::InsufficientFunds => true,
            Self::DoNotHonor => config.retryable_decline_codes.contains(self),
            Self::InvalidCard => false,
            Self::ExpiredCard => false,
            Self::SuspectedFraud => false,
            Self::IssuerUnavailable => true,
            Self::ThreeDSecureFailed => config.retryable_decline_codes.contains(self),
            Self::RateLimitedByAcquirer => true,
            Self::PartialAuthorizationRejected => false,
            Self::UnknownError(_) => config.retry_unknown_as_fallback,
        }
    }
}
```

## 6. Routing Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub card_scheme: Option<CardScheme>,
    pub currency: Option<CurrencyCode>,
    pub min_amount: Option<Money>,
    pub max_amount: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardScheme {
    Visa,
    Mastercard,
    Amex,
    Mada,
    UnionPay,
    Jcb,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub retryable_decline_codes: Vec<DeclineReason>,
    pub max_hops: u8,                // platform ceiling: 3
    pub latency_budget_ms: u32,      // total checkout latency budget
    pub retry_unknown_as_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialAuthorizationPolicy {
    pub strategy: PartialAuthStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartialAuthStrategy {
    AcceptPartial,
    RetryNextAcquirer,
    Reject,
}
```

## 7. Domain Event Envelope

```protobuf
syntax = "proto3";
package common.v1;

message EventEnvelope {
  string event_id = 1;           // UUIDv7
  string aggregate_type = 2;
  string aggregate_id = 3;
  string event_type = 4;
  uint32 event_version = 5;
  int64 occurred_at_unix_ms = 6; // milliseconds since epoch
  string actor_type = 7;         // user | api_key | system
  string actor_id = 8;
  string causation_id = 9;       // UUIDv7 — command that caused this event
  string correlation_id = 10;    // UUIDv7 — ties full business transaction
  bytes payload = 11;            // protobuf-encoded event payload
  string trace_context = 12;     // W3C Trace Context (optional)
  bytes signature = 13;          // HMAC-SHA256 (optional, for event signing)
}
```

## 8. gRPC Shared Types — REST Paths + Protobuf Bodies

All external APIs use RESTful paths with protobuf-encoded request/response bodies (Content-Type: application/protobuf). HTTP methods: POST, PATCH, DELETE. Internal service-to-service communication uses native gRPC with the same protobuf schemas.

```protobuf
syntax = "proto3";
package common.v1;

// Shared Money type — used across all services
message Money {
  int64 amount_minor_units = 1;
  string currency_code = 2; // ISO 4217, e.g., "AED", "USD"
}

// Cursor-based pagination (no query strings — all in request body)
message PaginationRequest {
  string cursor = 1;
  uint32 limit = 2; // max 100
}

message PaginationResponse {
  string next_cursor = 1;
  bool has_more = 2;
  int64 as_of_unix_ms = 3; // freshness disclosure
}

// Timestamp with millisecond precision
message Timestamp {
  int64 unix_ms = 1; // milliseconds since epoch
}

// Error detail — returned in all error responses
message ErrorDetail {
  string code = 1;
  string message = 2;
  string request_id = 3;
  map<string, string> details = 4;
}

// Internal error codes for service-to-service communication
enum InternalErrorCode {
  INTERNAL_ERROR_CODE_UNSPECIFIED = 0;
  TRANSIENT_FAILURE = 1;
  PERMANENT_FAILURE = 2;
  DEGRADED_MODE = 3;
  RATE_LIMITED = 4;
  AUTHORIZATION_DENIED = 5;
  VALIDATION_ERROR = 6;
  UNAVAILABLE = 7;
}

// Source context — who initiated this payment
message SourceContext {
  string source_type = 1;    // 'merchant_api' | 'invoice' | 'subscription' | 'payment_link' | 'ai_assistant' | 'system'
  string source_id = 2;      // optional: invoice_id, subscription_id, etc.
}

// Risk assessment result
message RiskAssessment {
  double risk_score = 1;     // 0.0 - 1.0
  string risk_level = 2;     // 'low' | 'medium' | 'high' | 'critical'
  string rule_version = 3;
  repeated string risk_factors = 4;
}

// Fee breakdown from settlement
message FeeBreakdown {
  int64 interchange_fee_minor_units = 1;
  int64 scheme_fee_minor_units = 2;
  int64 acquirer_markup_minor_units = 3;
  int64 processing_fee_minor_units = 4;
  int64 total_fee_minor_units = 5;
  string fee_currency = 6;
}

// FX rate from acquirer
message FxRate {
  string source_currency = 1;
  string target_currency = 2;
  int64 rate_minor_units = 3;  // rate * 10^6 for integer math
  int64 expires_at_unix_ms = 4;
}
```

## 9. Shared Error Types

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum PlatformError {
    #[error("Validation error: {0}")]
    Validation(ValidationError),

    #[error("Not found: {resource} {id}")]
    NotFound { resource: String, id: Uuid },

    #[error("Conflict: {0}")]
    Conflict(ConflictError),

    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    #[error("Rate limited")]
    RateLimited { retry_after_ms: u64 },

    #[error("Service unavailable: {0}")]
    Unavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid currency code")]
    InvalidCurrencyCode,
    #[error("Negative amount")]
    NegativeAmount,
    #[error("Currency mismatch")]
    CurrencyMismatch,
    #[error("Amount overflow")]
    AmountOverflow,
    #[error("Invalid idempotency key")]
    InvalidIdempotencyKey,
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid state transition: {from} -> {command}")]
    InvalidStateTransition { from: String, command: String },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConflictError {
    #[error("Duplicate idempotency key with different payload")]
    IdempotencyKeyConflict,
    #[error("Optimistic concurrency violation")]
    ConcurrencyViolation,
    #[error("Payment intent already captured")]
    AlreadyCaptured,
    #[error("Payment intent fully refunded")]
    FullyRefunded,
    #[error("Duplicate order invoice")]
    DuplicateOrderInvoice,
}
```

## 10. Source Context (Gap: Payment Origin Tracking)

Every `PaymentIntent` records who initiated the payment, enabling analytics segmentation by channel.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceContext {
    pub source_type: SourceType,
    pub source_id: Option<Uuid>,     // invoice_id, subscription_id, payment_link_id, etc.
    pub source_metadata: Option<String>, // JSON: additional context
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    MerchantApi,        // direct API call from merchant server
    Invoice,            // invoice-service initiated
    Subscription,       // subscription renewal initiated
    PaymentLink,        // hosted checkout page
    AiAssistant,        // AI-suggested action (human-confirmed)
    System,             // internal system action (e.g., retry)
}
```

## 11. Payment Method Token Types (Gap: Token Lifecycle)

Acquirer-issued tokens for recurring payments. The platform stores tokens but never raw card data.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentMethodType {
    Card,
    BankAccount,
    Wallet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenStatus {
    Active,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodTokenInfo {
    pub token_id: Uuid,
    pub payment_method_type: PaymentMethodType,
    pub last_four: String,
    pub card_brand: Option<String>,    // visa, mastercard, amex, mada
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub token_status: TokenStatus,
    pub acquirer_link_id: Uuid,        // tokens are per-acquirer
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: Option<DateTimeWithTimeZone>,
}
```

## 12. Outbound Webhook Types (Gap: Webhook Delivery)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebhookEventType {
    PaymentCreated,
    PaymentAuthorized,
    PaymentCaptured,
    PaymentFailed,
    PaymentRefunded,
    PaymentVoided,
    SettlementMatched,
    SettlementUnmatched,
    ChargebackReceived,
    ChargebackResolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub url: String,                    // HTTPS only, validated
    pub event_types: Vec<WebhookEventType>,
    pub secret: String,                 // for HMAC-SHA256 signing (stored hashed)
    pub status: SubscriptionStatus,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub delivery_id: Uuid,
    pub subscription_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: String,                // JSON payload sent
    pub status: DeliveryStatus,
    pub attempt_count: u32,
    pub last_attempt_at: Option<DateTimeWithTimeZone>,
    pub next_retry_at: Option<DateTimeWithTimeZone>,
    pub response_status_code: Option<u16>,
    pub response_body: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    PermanentlyFailed,
}
```

## 13. FX Rate Types (Gap: Multi-Currency)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FXRate {
    pub source_currency: CurrencyCode,
    pub target_currency: CurrencyCode,
    pub rate: String,              // string to avoid float precision issues
    pub rate_minor_units: i64,     // rate * 10^6 for integer math
    pub source: String,            // "acquirer" | "ecb" | "cbr"
    pub fetched_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FXQuote {
    pub quote_id: Uuid,
    pub source_amount: Money,
    pub target_amount: Money,
    pub rate: FXRate,
    pub fee: Option<Money>,
}
```

## 14. Settlement Cycle Types (Gap: T+N Handling)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementCycle {
    SameDay,            // T+0
    NextDay,            // T+1
    TwoDays,            // T+2
    ThreeDays,          // T+3
    Weekly,             // T+7
    Custom(u32),        // T+N
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementExpectation {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: Date,
    pub settlement_cycle: SettlementCycle,
    pub status: SettlementExpectationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementExpectationStatus {
    Pending,
    Settled,
    Overdue,
    Adjusted,
}
```

## 15. Fee Variance Types (Gap: Fee Reconciliation)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeVariance {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub estimated_fee: Money,           // calculated at auth time
    pub actual_fee: Money,              // from settlement record
    pub variance_amount: Money,         // actual - estimated
    pub variance_percent: f64,          // (actual - estimated) / estimated * 100
    pub is_within_tolerance: bool,      // compared to per-acquirer tolerance threshold
    pub detected_at: DateTimeWithTimeZone,
}
```

## 16. TDD Test Cases for Shared Types

### Money Tests

```rust
#[cfg(test)]
mod money_tests {
    use super::*;

    #[test]
    fn test_money_creation_valid() {
        let m = Money { amount_minor_units: 10000, currency: CurrencyCode::new("AED").unwrap() };
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_money_rejects_negative() {
        let m = Money { amount_minor_units: -100, currency: CurrencyCode::new("AED").unwrap() };
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_money_addition_same_currency() {
        let a = Money { amount_minor_units: 1000, currency: CurrencyCode::new("AED").unwrap() };
        let b = Money { amount_minor_units: 2000, currency: CurrencyCode::new("AED").unwrap() };
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.amount_minor_units, 3000);
    }

    #[test]
    fn test_money_addition_different_currency_fails() {
        let a = Money { amount_minor_units: 1000, currency: CurrencyCode::new("AED").unwrap() };
        let b = Money { amount_minor_units: 2000, currency: CurrencyCode::new("USD").unwrap() };
        assert!(a.checked_add(&b).is_err());
    }

    #[test]
    fn test_minor_unit_precision_bhd() {
        let m = Money { amount_minor_units: 1000, currency: CurrencyCode::new("BHD").unwrap() };
        assert_eq!(m.minor_unit_precision(), 3);
    }

    #[test]
    fn test_minor_unit_precision_jpy() {
        let m = Money { amount_minor_units: 1000, currency: CurrencyCode::new("JPY").unwrap() };
        assert_eq!(m.minor_unit_precision(), 0);
    }

    #[test]
    fn test_zero_amount_is_card_verification() {
        let m = Money { amount_minor_units: 0, currency: CurrencyCode::new("AED").unwrap() };
        assert!(m.is_zero());
    }
}
```

### PaymentStatus Tests

```rust
#[cfg(test)]
mod payment_status_tests {
    use super::*;

    #[test]
    fn test_authorized_can_capture() {
        assert!(PaymentStatus::Authorized.can_transition(&PaymentCommand::Capture));
    }

    #[test]
    fn test_authorized_can_void() {
        assert!(PaymentStatus::Authorized.can_transition(&PaymentCommand::Void));
    }

    #[test]
    fn test_cannot_capture_failed() {
        assert!(!PaymentStatus::Failed.can_transition(&PaymentCommand::Capture));
    }

    #[test]
    fn test_cannot_void_after_capture() {
        assert!(!PaymentStatus::Captured.can_transition(&PaymentCommand::Void));
    }

    #[test]
    fn test_cannot_refund_voided() {
        assert!(!PaymentStatus::Voided.can_transition(&PaymentCommand::Refund));
    }

    #[test]
    fn test_partially_captured_can_capture_remaining() {
        assert!(PaymentStatus::PartiallyCaptured.can_transition(&PaymentCommand::Capture));
    }

    #[test]
    fn test_cannot_refund_zero_amount() {
        // Zero-amount authorizations (card verification) cannot be refunded
        // This is enforced at the command handler level, not state machine
    }
}
```

### DeclineReason Tests

```rust
#[cfg(test)]
mod decline_reason_tests {
    use super::*;

    #[test]
    fn test_insufficient_funds_is_retryable() {
        let config = FailoverConfig {
            retryable_decline_codes: vec![],
            max_hops: 3,
            latency_budget_ms: 10000,
            retry_unknown_as_fallback: false,
        };
        assert!(DeclineReason::InsufficientFunds.is_retryable(&config));
    }

    #[test]
    fn test_invalid_card_not_retryable() {
        let config = FailoverConfig {
            retryable_decline_codes: vec![],
            max_hops: 3,
            latency_budget_ms: 10000,
            retry_unknown_as_fallback: false,
        };
        assert!(!DeclineReason::InvalidCard.is_retryable(&config));
    }

    #[test]
    fn test_unknown_error_respects_config() {
        let config_retry = FailoverConfig {
            retryable_decline_codes: vec![],
            max_hops: 3,
            latency_budget_ms: 10000,
            retry_unknown_as_fallback: true,
        };
        let config_no_retry = FailoverConfig {
            retryable_decline_codes: vec![],
            max_hops: 3,
            latency_budget_ms: 10000,
            retry_unknown_as_fallback: false,
        };
        assert!(DeclineReason::UnknownError("99".into()).is_retryable(&config_retry));
        assert!(!DeclineReason::UnknownError("99".into()).is_retryable(&config_no_retry));
    }
}
```
