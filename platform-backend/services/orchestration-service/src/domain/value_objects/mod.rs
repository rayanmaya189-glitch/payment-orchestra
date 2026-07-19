#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};
use shared_types::{CardScheme, CurrencyCode, Money};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentPurpose {
    Payment,
    CardVerification,
}

impl PaymentPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Payment => "payment",
            Self::CardVerification => "card_verification",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "card_verification" => Self::CardVerification,
            _ => Self::Payment,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub interchange: i64,
    pub scheme: i64,
    pub acquirer_markup: i64,
    pub processing: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub rule_id: String,
    pub priority: i32,
    pub card_scheme: Option<CardScheme>,
    pub currency: Option<CurrencyCode>,
    pub min_amount: Option<i64>,
    pub max_amount: Option<i64>,
    pub acquirer_link_id: uuid::Uuid,
}

impl RoutingRule {
    pub fn matches(
        &self,
        card_scheme: Option<&CardScheme>,
        currency: &CurrencyCode,
        amount: &Money,
    ) -> bool {
        if let Some(ref required_scheme) = self.card_scheme {
            if let Some(actual_scheme) = card_scheme {
                if required_scheme != actual_scheme {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref required_currency) = self.currency {
            if required_currency != currency {
                return false;
            }
        }

        if let Some(min) = self.min_amount {
            if amount.amount_minor_units < min {
                return false;
            }
        }

        if let Some(max) = self.max_amount {
            if amount.amount_minor_units > max {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub retryable_decline_codes: Vec<String>,
    pub max_hops: u8,
    pub latency_budget_ms: u32,
    pub retry_unknown_as_fallback: bool,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            retryable_decline_codes: vec!["insufficient_funds".to_string()],
            max_hops: 3,
            latency_budget_ms: 10000,
            retry_unknown_as_fallback: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RouteSelection {
    Acquirer(uuid::Uuid),
    NoRoute,
}
