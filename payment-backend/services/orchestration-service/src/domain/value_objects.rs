use serde::{Deserialize, Serialize};

use super::error::OrchestrationError;

// ─── Money ───────────────────────────────────────────────────────────────────

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

// ─── DeclineReason ───────────────────────────────────────────────────────────

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

// ─── Fee Breakdown ───────────────────────────────────────────────────────────

/// Fee breakdown for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub interchange_minor: i64,
    pub scheme_fee_minor: i64,
    pub acquirer_markup_minor: i64,
    pub processing_fee_minor: i64,
    pub total_minor: i64,
}

// ─── Payment Purpose ─────────────────────────────────────────────────────────

/// Payment purpose
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentPurpose {
    Payment,
    CardVerification,
}

// ─── Source Type ─────────────────────────────────────────────────────────────

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
