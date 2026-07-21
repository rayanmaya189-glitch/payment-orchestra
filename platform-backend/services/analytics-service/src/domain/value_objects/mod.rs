use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, String> {
        if start >= end {
            return Err("Start time must be before end time".into());
        }
        let duration = end.signed_duration_since(start);
        if duration.num_days() > 365 {
            return Err("Time range cannot exceed 365 days".into());
        }
        Ok(Self { start, end })
    }

    pub fn last_24h() -> Self {
        Self {
            start: Utc::now() - chrono::Duration::hours(24),
            end: Utc::now(),
        }
    }

    pub fn last_7d() -> Self {
        Self {
            start: Utc::now() - chrono::Duration::days(7),
            end: Utc::now(),
        }
    }

    pub fn last_30d() -> Self {
        Self {
            start: Utc::now() - chrono::Duration::days(30),
            end: Utc::now(),
        }
    }

    pub fn duration_hours(&self) -> f64 {
        self.end
            .signed_duration_since(self.start)
            .num_minutes() as f64
            / 60.0
    }

    pub fn duration_days(&self) -> f64 {
        self.duration_hours() / 24.0
    }

    pub fn parse(start_str: &str, end_str: &str) -> Result<Self, String> {
        let start = DateTime::parse_from_rfc3339(start_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| format!("Invalid start time: {e}"))?;
        let end = DateTime::parse_from_rfc3339(end_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| format!("Invalid end time: {e}"))?;
        Self::new(start, end)
    }
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} to {}", self.start.to_rfc3339(), self.end.to_rfc3339())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregationType {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

impl AggregationType {
    pub fn from_hours(hours: f64) -> Self {
        if hours <= 48.0 {
            Self::Hourly
        } else if hours <= 30.0 * 24.0 {
            Self::Daily
        } else if hours <= 90.0 * 24.0 {
            Self::Weekly
        } else {
            Self::Monthly
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsMetric {
    Volume,
    SuccessRate,
    Latency,
    Revenue,
    DeclineRate,
    AverageTransactionValue,
    RefundRate,
}

impl AnalyticsMetric {
    pub fn label(&self) -> &str {
        match self {
            Self::Volume => "volume",
            Self::SuccessRate => "success_rate",
            Self::Latency => "latency",
            Self::Revenue => "revenue",
            Self::DeclineRate => "decline_rate",
            Self::AverageTransactionValue => "avg_transaction_value",
            Self::RefundRate => "refund_rate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclineCategory {
    DoNotHonor,
    InsufficientFunds,
    ExpiredCard,
    RestrictedCard,
    SuspectedFraud,
    IncorrectCvc,
    ExceedsLimit,
    MerchantRequest,
    DoNotTryAgain,
    InvalidTransaction,
    LimitExceeded,
    ServiceUnavailable,
    Other,
}

impl DeclineCategory {
    pub fn from_code(code: &str) -> Self {
        match code {
            "05" | "58" => Self::DoNotHonor,
            "51" => Self::InsufficientFunds,
            "54" => Self::ExpiredCard,
            "36" | "43" => Self::RestrictedCard,
            "59" | "63" => Self::SuspectedFraud,
            "65" => Self::IncorrectCvc,
            "61" => Self::ExceedsLimit,
            "57" => Self::MerchantRequest,
            "41" => Self::DoNotTryAgain,
            "12" => Self::InvalidTransaction,
            "65" => Self::LimitExceeded,
            "91" | "96" => Self::ServiceUnavailable,
            _ => Self::Other,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::DoNotHonor => "do_not_honor",
            Self::InsufficientFunds => "insufficient_funds",
            Self::ExpiredCard => "expired_card",
            Self::RestrictedCard => "restricted_card",
            Self::SuspectedFraud => "suspected_fraud",
            Self::IncorrectCvc => "incorrect_cvc",
            Self::ExceedsLimit => "exceeds_limit",
            Self::MerchantRequest => "merchant_request",
            Self::DoNotTryAgain => "do_not_try_again",
            Self::InvalidTransaction => "invalid_transaction",
            Self::LimitExceeded => "limit_exceeded",
            Self::ServiceUnavailable => "service_unavailable",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurrencyAmount {
    pub amount_minor_units: i64,
    pub currency: String,
}

impl CurrencyAmount {
    pub fn new(amount: i64, currency: &str) -> Result<Self, String> {
        if amount < 0 {
            return Err("Amount cannot be negative".into());
        }
        if currency.len() != 3 {
            return Err("Currency must be 3 characters".into());
        }
        Ok(Self {
            amount_minor_units: amount,
            currency: currency.to_uppercase(),
        })
    }

    pub fn as_major_units(&self) -> f64 {
        self.amount_minor_units as f64 / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_creation() {
        let now = Utc::now();
        let start = now - chrono::Duration::hours(24);
        let range = TimeRange::new(start, now);
        assert!(range.is_ok());
        let r = range.unwrap();
        assert!((r.duration_hours() - 24.0).abs() < 0.1);
    }

    #[test]
    fn test_time_range_validation() {
        let now = Utc::now();
        let range = TimeRange::new(now, now - chrono::Duration::hours(1));
        assert!(range.is_err());
    }

    #[test]
    fn test_time_range_defaults() {
        let range = TimeRange::last_24h();
        assert!(range.duration_hours() >= 23.9 && range.duration_hours() <= 24.1);
    }

    #[test]
    fn test_aggregation_type() {
        assert_eq!(AggregationType::from_hours(12.0), AggregationType::Hourly);
        assert_eq!(AggregationType::from_hours(72.0), AggregationType::Daily);
        assert_eq!(AggregationType::from_hours(60.0 * 24.0), AggregationType::Weekly);
    }

    #[test]
    fn test_decline_category() {
        assert_eq!(DeclineCategory::from_code("51"), DeclineCategory::InsufficientFunds);
        assert_eq!(DeclineCategory::from_code("54"), DeclineCategory::ExpiredCard);
        assert_eq!(DeclineCategory::from_code("59"), DeclineCategory::SuspectedFraud);
        assert_eq!(DeclineCategory::from_code("99"), DeclineCategory::Other);
    }

    #[test]
    fn test_currency_amount() {
        let amt = CurrencyAmount::new(1500, "usd").unwrap();
        assert_eq!(amt.as_major_units(), 15.0);
        assert!(CurrencyAmount::new(-1, "usd").is_err());
        assert!(CurrencyAmount::new(100, "us").is_err());
    }
}
