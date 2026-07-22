//! Orchestration-service domain model — the core of the payment engine.
//! Event-sourced aggregate: PaymentIntent.
//! CRUD + events aggregates: RoutingPolicy, PaymentMethodToken.
//!
//! Pure Router: This platform NEVER holds funds. It routes transaction *instructions*
//! between merchants, their acquirers/PSPs, and their customers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Re-exports ──────────────────────────────────────────────────────────────

pub use crate::events::*;

// ─── Value Objects ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: String, // ISO 4217: "AED", "USD", etc.
}

impl Money {
    pub fn zero(currency: &str) -> Self {
        Self { amount_minor_units: 0, currency: currency.to_string() }
    }

    pub fn is_zero(&self) -> bool {
        self.amount_minor_units == 0
    }

    pub fn checked_add(&self, other: &Money) -> Result<Self, OrchestrationError> {
        if self.currency != other.currency {
            return Err(OrchestrationError::Validation("Currency mismatch".into()));
        }
        let sum = self.amount_minor_units.checked_add(other.amount_minor_units)
            .ok_or_else(|| OrchestrationError::Validation("Amount overflow".into()))?;
        Ok(Self { amount_minor_units: sum, currency: self.currency.clone() })
    }

    pub fn checked_sub(&self, other: &Money) -> Result<Self, OrchestrationError> {
        if self.currency != other.currency {
            return Err(OrchestrationError::Validation("Currency mismatch".into()));
        }
        if self.amount_minor_units < other.amount_minor_units {
            return Err(OrchestrationError::Validation("Insufficient amount".into()));
        }
        Ok(Self { amount_minor_units: self.amount_minor_units - other.amount_minor_units, currency: self.currency.clone() })
    }
}

/// Normalized decline reasons across all acquirers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DeclineReason {
    InsufficientFunds,
    DoNotHonor,
    InvalidCard,
    ExpiredCard,
    SuspectedFraud,
    IssuerUnavailable,
    ThreeDSecureFailed,
    RateLimitedByAcquirer,
    InvalidAmount,
    LostCard,
    StolenCard,
    PickupCard,
    TransactionNotPermitted,
    ExceedsWithdrawalLimit,
    SecurityViolation,
    SystemError,
    Unknown(String),
}

impl DeclineReason {
    pub fn is_retryable(&self) -> bool {
        matches!(self, DeclineReason::InsufficientFunds
            | DeclineReason::DoNotHonor
            | DeclineReason::IssuerUnavailable
            | DeclineReason::RateLimitedByAcquirer
            | DeclineReason::SystemError
        )
    }
}

impl std::fmt::Display for DeclineReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeclineReason::InsufficientFunds => write!(f, "InsufficientFunds"),
            DeclineReason::DoNotHonor => write!(f, "DoNotHonor"),
            DeclineReason::InvalidCard => write!(f, "InvalidCard"),
            DeclineReason::ExpiredCard => write!(f, "ExpiredCard"),
            DeclineReason::SuspectedFraud => write!(f, "SuspectedFraud"),
            DeclineReason::IssuerUnavailable => write!(f, "IssuerUnavailable"),
            DeclineReason::ThreeDSecureFailed => write!(f, "ThreeDSecureFailed"),
            DeclineReason::RateLimitedByAcquirer => write!(f, "RateLimitedByAcquirer"),
            DeclineReason::InvalidAmount => write!(f, "InvalidAmount"),
            DeclineReason::LostCard => write!(f, "LostCard"),
            DeclineReason::StolenCard => write!(f, "StolenCard"),
            DeclineReason::PickupCard => write!(f, "PickupCard"),
            DeclineReason::TransactionNotPermitted => write!(f, "TransactionNotPermitted"),
            DeclineReason::ExceedsWithdrawalLimit => write!(f, "ExceedsWithdrawalLimit"),
            DeclineReason::SecurityViolation => write!(f, "SecurityViolation"),
            DeclineReason::SystemError => write!(f, "SystemError"),
            DeclineReason::Unknown(s) => write!(f, "Unknown({})", s),
        }
    }
}

impl From<&str> for DeclineReason {
    fn from(s: &str) -> Self {
        match s {
            "InsufficientFunds" => DeclineReason::InsufficientFunds,
            "DoNotHonor" => DeclineReason::DoNotHonor,
            "InvalidCard" => DeclineReason::InvalidCard,
            "ExpiredCard" => DeclineReason::ExpiredCard,
            "SuspectedFraud" => DeclineReason::SuspectedFraud,
            "IssuerUnavailable" => DeclineReason::IssuerUnavailable,
            "ThreeDSecureFailed" => DeclineReason::ThreeDSecureFailed,
            "RateLimitedByAcquirer" => DeclineReason::RateLimitedByAcquirer,
            "InvalidAmount" => DeclineReason::InvalidAmount,
            "LostCard" => DeclineReason::LostCard,
            "StolenCard" => DeclineReason::StolenCard,
            "PickupCard" => DeclineReason::PickupCard,
            "TransactionNotPermitted" => DeclineReason::TransactionNotPermitted,
            "ExceedsWithdrawalLimit" => DeclineReason::ExceedsWithdrawalLimit,
            "SecurityViolation" => DeclineReason::SecurityViolation,
            "SystemError" => DeclineReason::SystemError,
            other => DeclineReason::Unknown(other.to_string()),
        }
    }
}

/// Fee breakdown for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub interchange_minor: i64,
    pub scheme_fee_minor: i64,
    pub acquirer_markup_minor: i64,
    pub processing_fee_minor: i64,
    pub total_minor: i64,
}

/// Payment purpose
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentPurpose {
    Payment,
    CardVerification,
}

/// Source of the payment (for analytics segmentation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    MerchantApi,
    Invoice,
    Subscription,
    PaymentLink,
    AiAssistant,
    System,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::MerchantApi => write!(f, "merchant_api"),
            SourceType::Invoice => write!(f, "invoice"),
            SourceType::Subscription => write!(f, "subscription"),
            SourceType::PaymentLink => write!(f, "payment_link"),
            SourceType::AiAssistant => write!(f, "ai_assistant"),
            SourceType::System => write!(f, "system"),
        }
    }
}

// ─── Payment Status State Machine ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
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

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PaymentStatus {
    /// Validate a state transition for a given command.
    /// Returns Ok(()) if the transition is valid, Err with the appropriate error code otherwise.
    pub fn can_execute_command(&self, command: &str) -> Result<(), OrchestrationError> {
        match (self, command) {
            // Valid transitions
            (Self::Created, "Authorize") => Ok(()),
            (Self::Authorizing, "Authorize") => Ok(()),
            (Self::Authorized, "Capture") => Ok(()),
            (Self::Authorized, "Void") => Ok(()),
            (Self::PartiallyCaptured, "Capture") => Ok(()),
            (Self::Captured, "Refund") => Ok(()),
            (Self::PartiallyCaptured, "Refund") => Ok(()),

            // Invalid transitions with specific error codes
            (Self::Failed, cmd) | (Self::FailedAllRoutes, cmd) if cmd == "Capture" || cmd == "Void" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_FAILED".into())),
            (Self::Voided, cmd) if cmd == "Capture" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_VOIDED".into())),
            (Self::AuthorizationExpired, cmd) if cmd == "Capture" || cmd == "Void" =>
                Err(OrchestrationError::InvalidStateTransition("AUTHORIZATION_EXPIRED".into())),
            (Self::Captured, cmd) if cmd == "Capture" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_ALREADY_CAPTURED".into())),
            (Self::Refunded, cmd) if cmd == "Refund" || cmd == "Capture" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_FULLY_REFUNDED".into())),
            (Self::Authorizing, cmd) if cmd == "Capture" || cmd == "Void" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_AUTHORIZING".into())),
            (Self::Capturing, cmd) if cmd == "Void" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_CAPTURING".into())),
            (Self::Created, cmd) if cmd == "Capture" || cmd == "Void" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_NOT_AUTHORIZED".into())),

            // Default: transition not defined
            _ => Err(OrchestrationError::InvalidStateTransition(format!("Cannot {} in state {:?}", command, self))),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Captured | Self::Voided | Self::AuthorizationExpired
            | Self::Failed | Self::FailedAllRoutes | Self::Refunded)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed | Self::FailedAllRoutes)
    }
}

// ─── AGG-01: PaymentIntent (Aggregate Root, Event-Sourced) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingAttempt {
    pub attempt_id: Uuid,
    pub payment_intent_id: Uuid,
    pub attempt_number: i32,
    pub acquirer_link_id: Uuid,
    pub gateway_profile_id: Option<Uuid>,
    pub gateway_profile_snapshot: Option<String>,
    pub connector_id: String,
    pub status: AttemptStatus,
    pub decline_reason: Option<DeclineReason>,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
    pub fee_calculated: Option<String>,
    pub attempted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttemptStatus {
    Approved,
    Declined,
    Timeout,
    Requires3DS,
    PartialApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub status: PaymentStatus,
    pub requested_amount: Money,
    pub authorized_amount: Money,
    pub captured_amount: Money,
    pub refunded_amount: Money,
    pub currency: String,
    pub idempotency_key: String,
    pub payment_method_token_id: Option<Uuid>,
    pub routing_policy_id: Option<Uuid>,
    pub deployment_epoch: i32,
    pub purpose: PaymentPurpose,
    pub metadata: Option<serde_json::Value>,

    // Source Context
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,

    // Risk Score
    pub risk_score: Option<f64>,
    pub risk_level: Option<String>,

    // Settlement Timing
    pub expected_settlement_date: Option<String>,
    pub settlement_cycle: Option<String>,

    // Gateway Profile Link
    pub gateway_profile_id: Option<Uuid>,
    pub gateway_profile_version: Option<i32>,
    pub gateway_rotation_strategy: Option<String>,
    pub gateway_selection_reason: Option<String>,

    // Routing attempts (history)
    pub routing_attempts: Vec<RoutingAttempt>,

    // Event-sourcing version
    pub version: i64,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PaymentIntent {
    pub fn new(
        payment_intent_id: Uuid,
        operator_id: Uuid,
        amount: Money,
        purpose: PaymentPurpose,
        idempotency_key: String,
        source_type: SourceType,
        source_id: Option<Uuid>,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            payment_intent_id,
            operator_id,
            status: PaymentStatus::Created,
            requested_amount: amount.clone(),
            authorized_amount: Money::zero(&amount.currency),
            captured_amount: Money::zero(&amount.currency),
            refunded_amount: Money::zero(&amount.currency),
            currency: amount.currency.clone(),
            idempotency_key,
            payment_method_token_id: None,
            routing_policy_id: None,
            deployment_epoch: 0,
            purpose,
            metadata,
            source_type: Some(source_type.to_string()),
            source_id,
            risk_score: None,
            risk_level: None,
            expected_settlement_date: None,
            settlement_cycle: None,
            gateway_profile_id: None,
            gateway_profile_version: None,
            gateway_rotation_strategy: None,
            gateway_selection_reason: None,
            routing_attempts: Vec::new(),
            version: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Apply a state transition event to evolve the aggregate.
    pub fn apply_event(&mut self, event: &PaymentEvent) {
        match event {
            PaymentEvent::PaymentIntentCreated(e) => {
                self.status = PaymentStatus::Created;
                self.requested_amount = Money { amount_minor_units: e.amount_minor_units, currency: e.currency.clone() };
                self.authorized_amount = Money::zero(&e.currency);
                self.captured_amount = Money::zero(&e.currency);
                self.refunded_amount = Money::zero(&e.currency);
                self.currency = e.currency.clone();
                self.source_type = e.source_type.clone();
                self.source_id = e.source_id;
                self.purpose = if e.is_card_verification { PaymentPurpose::CardVerification } else { PaymentPurpose::Payment };
                self.created_at = e.occurred_at;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentAuthorizationAttempted(e) => {
                self.status = PaymentStatus::Authorizing;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentAuthorized(e) => {
                self.status = PaymentStatus::Authorized;
                self.authorized_amount = Money { amount_minor_units: e.authorized_amount_minor, currency: self.currency.clone() };
                self.gateway_profile_id = e.gateway_profile_id;
                self.gateway_profile_version = e.gateway_profile_version;
                self.gateway_rotation_strategy = e.rotation_strategy.clone();
                self.gateway_selection_reason = e.selection_reason.clone();
                self.expected_settlement_date = e.expected_settlement_date.clone();
                self.settlement_cycle = e.settlement_cycle.clone();
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentCaptured(e) => {
                self.status = PaymentStatus::Captured;
                let current_captured = self.captured_amount.amount_minor_units;
                self.captured_amount = Money {
                    amount_minor_units: current_captured + e.captured_amount_minor,
                    currency: self.currency.clone(),
                };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentPartiallyCaptured(e) => {
                self.status = PaymentStatus::PartiallyCaptured;
                let current_captured = self.captured_amount.amount_minor_units;
                self.captured_amount = Money {
                    amount_minor_units: current_captured + e.captured_amount_minor,
                    currency: self.currency.clone(),
                };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentFailed(e) => {
                self.status = PaymentStatus::Failed;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentFailedAllRoutes(e) => {
                self.status = PaymentStatus::FailedAllRoutes;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentVoided(e) => {
                self.status = PaymentStatus::Voided;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentRefunded(e) => {
                self.status = PaymentStatus::Refunded;
                self.refunded_amount = Money { amount_minor_units: e.refund_amount_minor, currency: self.currency.clone() };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentPartiallyRefunded(e) => {
                self.status = PaymentStatus::PartiallyRefunded;
                self.refunded_amount = Money { amount_minor_units: e.refund_amount_minor, currency: self.currency.clone() };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            _ => {} // Non-PaymentIntent events
        }
    }

    /// Check INV-01: captured_amount <= authorized_amount
    pub fn check_capture_invariant(&self, capture_amount: &Money) -> Result<(), OrchestrationError> {
        // Sum of existing captures + this capture must not exceed authorized
        let total_after = self.captured_amount.checked_add(capture_amount)?;
        if total_after.amount_minor_units > self.authorized_amount.amount_minor_units {
            return Err(OrchestrationError::InvariantViolation("INV-01: capture would exceed authorized amount".into()));
        }
        Ok(())
    }

    /// Check remaining refundable balance
    pub fn remaining_refundable(&self) -> Result<Money, OrchestrationError> {
        self.captured_amount.checked_sub(&self.refunded_amount)
    }
}

// ─── AGG-02: RoutingPolicy (Aggregate Root, CRUD + Events) ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub acquirer_link_id: Uuid,
    pub priority: i32,
    pub condition: RoutingCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub card_schemes: Option<Vec<String>>,
    pub currencies: Option<Vec<String>>,
    pub min_amount_minor: Option<i64>,
    pub max_amount_minor: Option<i64>,
}

impl RoutingCondition {
    pub fn all() -> Self {
        Self { card_schemes: None, currencies: None, min_amount_minor: None, max_amount_minor: None }
    }

    pub fn matches(&self, card_scheme: &str, currency: &str, amount_minor: i64) -> bool {
        if let Some(schemes) = &self.card_schemes {
            if !schemes.iter().any(|s| s == card_scheme) { return false; }
        }
        if let Some(currencies) = &self.currencies {
            if !currencies.iter().any(|c| c == currency) { return false; }
        }
        if let Some(min) = self.min_amount_minor {
            if amount_minor < min { return false; }
        }
        if let Some(max) = self.max_amount_minor {
            if amount_minor > max { return false; }
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    Priority,
    RoundRobin,
    WeightedRoundRobin { weights: Vec<(Uuid, u32)> },
    CostBased,
    SuccessRateBased,
    VolumeCapped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartialAuthStrategy {
    AcceptPartial,
    RetryNextAcquirer,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub max_hops: u8,
    pub latency_budget_ms: u32,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self { max_hops: 3, latency_budget_ms: 10000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub routing_policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub status: PolicyStatus,
    pub rules: Vec<RoutingRule>,
    pub failover_config: FailoverConfig,
    pub partial_auth_strategy: PartialAuthStrategy,
    pub rotation_strategy: RotationStrategy,
    pub max_transaction_amount_minor: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyStatus {
    Active,
    Inactive,
}

impl RoutingPolicy {
    pub fn new(routing_policy_id: Uuid, operator_id: Uuid, rules: Vec<RoutingRule>) -> Self {
        Self {
            routing_policy_id,
            operator_id,
            version: 1,
            status: PolicyStatus::Inactive,
            rules,
            failover_config: FailoverConfig::default(),
            partial_auth_strategy: PartialAuthStrategy::AcceptPartial,
            rotation_strategy: RotationStrategy::Priority,
            max_transaction_amount_minor: None,
            created_at: Utc::now(),
            activated_at: None,
        }
    }

    /// Select a gateway profile based on routing rules and conditions.
    pub fn select_route(
        &self,
        card_scheme: &str,
        currency: &str,
        amount_minor: i64,
        attempted_hops: &[Uuid],
        available_links: &[Uuid],
    ) -> Result<Uuid, OrchestrationError> {
        for rule in &self.rules {
            if !rule.condition.matches(card_scheme, currency, amount_minor) {
                continue;
            }
            if attempted_hops.contains(&rule.acquirer_link_id) {
                continue;
            }
            if !available_links.contains(&rule.acquirer_link_id) {
                continue;
            }
            return Ok(rule.acquirer_link_id);
        }
        Err(OrchestrationError::NoEligibleRoute)
    }
}

// ─── AGG-03: PaymentMethodToken (Aggregate Root, CRUD + Events) ──────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodToken {
    pub token_id: Uuid,
    pub operator_id: Uuid,
    pub payment_method_type: String,
    pub last_four: String,
    pub card_brand: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub token_status: TokenStatus,
    pub acquirer_link_id: Uuid,
    pub acquirer_token_reference: String,
    pub encrypted_token: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenStatus {
    Active,
    Expired,
    Revoked,
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error)]
pub enum OrchestrationError {
    #[error("PaymentIntent not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Concurrency conflict: expected version {expected}, got {actual}")]
    ConcurrencyConflict { expected: i64, actual: i64 },

    #[error("Idempotency conflict: key {0} used with different payload")]
    IdempotencyConflict(String),

    #[error("No eligible route found")]
    NoEligibleRoute,

    #[error("Routing policy not found")]
    RoutingPolicyNotFound,

    #[error("All acquirers declined")]
    AllAcquirersDeclined,

    #[error("Payment method token not found or inactive")]
    PaymentMethodTokenInvalid,
}

// ─── Idempotency ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IdempotencyRecord {
    pub key: String,
    pub payload_hash: Vec<u8>,
    pub result_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub enum IdempotencyResult {
    /// New idempotency key — caller should process
    New,
    /// Duplicate key with same payload — return cached result
    Duplicate(serde_json::Value),
    /// Same key, different payload — conflict error
    Conflict,
}
