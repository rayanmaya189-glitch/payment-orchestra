use serde::{Deserialize, Serialize};
use crate::money::{CurrencyCode, Money};
use crate::decline_reason::DeclineReason;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub card_scheme: Option<CardScheme>,
    pub currency: Option<CurrencyCode>,
    pub min_amount: Option<Money>,
    pub max_amount: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    pub max_hops: u8,
    pub latency_budget_ms: u32,
    pub retry_unknown_as_fallback: bool,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            retryable_decline_codes: vec![
                DeclineReason::InsufficientFunds,
                DeclineReason::DoNotHonor,
            ],
            max_hops: 3,
            latency_budget_ms: 10000,
            retry_unknown_as_fallback: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialAuthorizationPolicy {
    pub strategy: PartialAuthStrategy,
}

impl Default for PartialAuthorizationPolicy {
    fn default() -> Self {
        Self { strategy: PartialAuthStrategy::AcceptPartial }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartialAuthStrategy {
    AcceptPartial,
    RetryNextAcquirer,
    Reject,
}
