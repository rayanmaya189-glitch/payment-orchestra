use crate::decline_reason::DeclineReason;
use crate::money::{CurrencyCode, Money};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub card_scheme: Option<CardScheme>,
    pub currency: Option<CurrencyCode>,
    pub min_amount: Option<Money>,
    pub max_amount: Option<Money>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
