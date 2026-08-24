use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
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
    pub fn is_retryable(&self, config: &super::routing::FailoverConfig) -> bool {
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
    use crate::routing::FailoverConfig;

    #[test]
    fn test_insufficient_funds_is_retryable() {
        let config = FailoverConfig::default();
        assert!(DeclineReason::InsufficientFunds.is_retryable(&config));
    }

    #[test]
    fn test_invalid_card_not_retryable() {
        let config = FailoverConfig::default();
        assert!(!DeclineReason::InvalidCard.is_retryable(&config));
    }

    #[test]
    fn test_unknown_error_respects_config() {
        let config_retry = FailoverConfig { retry_unknown_as_fallback: true, ..FailoverConfig::default() };
        let config_no_retry = FailoverConfig { retry_unknown_as_fallback: false, ..FailoverConfig::default() };
        assert!(DeclineReason::UnknownError("99".into()).is_retryable(&config_retry));
        assert!(!DeclineReason::UnknownError("99".into()).is_retryable(&config_no_retry));
    }
}
