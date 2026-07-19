use crate::routing::FailoverConfig;
use serde::{Deserialize, Serialize};

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
    UnknownError(String),
}

impl DeclineReason {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> FailoverConfig {
        FailoverConfig {
            retryable_decline_codes: vec![],
            max_hops: 3,
            latency_budget_ms: 10000,
            retry_unknown_as_fallback: false,
        }
    }

    #[test]
    fn test_insufficient_funds_is_retryable() {
        assert!(DeclineReason::InsufficientFunds.is_retryable(&default_config()));
    }

    #[test]
    fn test_invalid_card_not_retryable() {
        assert!(!DeclineReason::InvalidCard.is_retryable(&default_config()));
    }

    #[test]
    fn test_unknown_error_respects_config() {
        let mut config_retry = default_config();
        config_retry.retry_unknown_as_fallback = true;
        assert!(DeclineReason::UnknownError("99".into()).is_retryable(&config_retry));
        assert!(!DeclineReason::UnknownError("99".into()).is_retryable(&default_config()));
    }

    #[test]
    fn test_issuer_unavailable_is_retryable() {
        assert!(DeclineReason::IssuerUnavailable.is_retryable(&default_config()));
    }

    #[test]
    fn test_suspected_fraud_not_retryable() {
        assert!(!DeclineReason::SuspectedFraud.is_retryable(&default_config()));
    }
}
