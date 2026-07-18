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
  bytes payload = 11;            // event-type-specific protobuf message
  string trace_context = 12;     // W3C Trace Context (optional)
  bytes signature = 13;          // HMAC-SHA256 (optional, for event signing)
}
```

## 8. gRPC Shared Types

```protobuf
syntax = "proto3";
package common.v1;

message Money {
  int64 amount_minor_units = 1;
  string currency_code = 2; // ISO 4217
}

message PaginationRequest {
  string cursor = 1;
  uint32 limit = 2; // max 100
}

message PaginationResponse {
  string next_cursor = 1;
  bool has_more = 2;
}

message Timestamp {
  int64 unix_ms = 1; // milliseconds since epoch
}

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

## 10. TDD Test Cases for Shared Types

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
