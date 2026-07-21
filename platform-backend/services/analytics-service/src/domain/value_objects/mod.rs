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

    // --- TimeRange::new ---

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
    fn test_time_range_equal_start_end_fails() {
        let now = Utc::now();
        let range = TimeRange::new(now, now);
        assert!(range.is_err());
        assert!(range.unwrap_err().contains("Start time must be before end time"));
    }

    #[test]
    fn test_time_range_exceeds_365_days_fails() {
        let now = Utc::now();
        let start = now - chrono::Duration::days(366);
        let range = TimeRange::new(start, now);
        assert!(range.is_err());
        assert!(range.unwrap_err().contains("365 days"));
    }

    #[test]
    fn test_time_range_exactly_365_days_succeeds() {
        let now = Utc::now();
        let start = now - chrono::Duration::days(365);
        let range = TimeRange::new(start, now);
        assert!(range.is_ok());
    }

    // --- TimeRange defaults ---

    #[test]
    fn test_time_range_defaults() {
        let range = TimeRange::last_24h();
        assert!(range.duration_hours() >= 23.9 && range.duration_hours() <= 24.1);
    }

    #[test]
    fn test_time_range_last_7d() {
        let range = TimeRange::last_7d();
        assert!(range.duration_hours() >= 167.9 && range.duration_hours() <= 168.1);
    }

    #[test]
    fn test_time_range_last_30d() {
        let range = TimeRange::last_30d();
        assert!(range.duration_hours() >= 719.9 && range.duration_hours() <= 720.1);
    }

    // --- TimeRange::parse ---

    #[test]
    fn test_time_range_parse_valid() {
        let range = TimeRange::parse("2025-01-01T00:00:00Z", "2025-01-02T00:00:00Z");
        assert!(range.is_ok());
        let r = range.unwrap();
        assert!((r.duration_hours() - 24.0).abs() < 0.1);
    }

    #[test]
    fn test_time_range_parse_invalid_start() {
        let range = TimeRange::parse("not-a-date", "2025-01-02T00:00:00Z");
        assert!(range.is_err());
        assert!(range.unwrap_err().contains("Invalid start time"));
    }

    #[test]
    fn test_time_range_parse_invalid_end() {
        let range = TimeRange::parse("2025-01-01T00:00:00Z", "not-a-date");
        assert!(range.is_err());
        assert!(range.unwrap_err().contains("Invalid end time"));
    }

    #[test]
    fn test_time_range_parse_with_timezone_offset() {
        let range = TimeRange::parse("2025-06-01T10:00:00+05:00", "2025-06-02T10:00:00+05:00");
        assert!(range.is_ok());
        let r = range.unwrap();
        assert!((r.duration_hours() - 24.0).abs() < 0.1);
    }

    #[test]
    fn test_time_range_parse_start_after_end_fails() {
        let range = TimeRange::parse("2025-01-02T00:00:00Z", "2025-01-01T00:00:00Z");
        assert!(range.is_err());
    }

    // --- TimeRange::duration ---

    #[test]
    fn test_time_range_duration_hours() {
        let now = Utc::now();
        let start = now - chrono::Duration::hours(48);
        let range = TimeRange::new(start, now).unwrap();
        assert!((range.duration_hours() - 48.0).abs() < 0.1);
    }

    #[test]
    fn test_time_range_duration_days() {
        let now = Utc::now();
        let start = now - chrono::Duration::days(7);
        let range = TimeRange::new(start, now).unwrap();
        assert!((range.duration_days() - 7.0).abs() < 0.01);
    }

    // --- TimeRange::Display ---

    #[test]
    fn test_time_range_display() {
        let range = TimeRange::parse("2025-01-01T00:00:00Z", "2025-01-02T00:00:00Z").unwrap();
        let display = format!("{range}");
        assert!(display.contains("2025-01-01"));
        assert!(display.contains("2025-01-02"));
    }

    // --- AggregationType ---

    #[test]
    fn test_aggregation_type() {
        assert_eq!(AggregationType::from_hours(12.0), AggregationType::Hourly);
        assert_eq!(AggregationType::from_hours(72.0), AggregationType::Daily);
        assert_eq!(AggregationType::from_hours(60.0 * 24.0), AggregationType::Weekly);
    }

    #[test]
    fn test_aggregation_type_hourly_boundary() {
        assert_eq!(AggregationType::from_hours(48.0), AggregationType::Hourly);
        assert_eq!(AggregationType::from_hours(49.0), AggregationType::Daily);
    }

    #[test]
    fn test_aggregation_type_daily_boundary() {
        assert_eq!(AggregationType::from_hours(720.0), AggregationType::Daily);
        assert_eq!(AggregationType::from_hours(721.0), AggregationType::Weekly);
    }

    #[test]
    fn test_aggregation_type_weekly_boundary() {
        assert_eq!(AggregationType::from_hours(2160.0), AggregationType::Weekly);
        assert_eq!(AggregationType::from_hours(2161.0), AggregationType::Monthly);
    }

    #[test]
    fn test_aggregation_type_labels() {
        assert_eq!(AggregationType::Hourly.label(), "hourly");
        assert_eq!(AggregationType::Daily.label(), "daily");
        assert_eq!(AggregationType::Weekly.label(), "weekly");
        assert_eq!(AggregationType::Monthly.label(), "monthly");
    }

    // --- DeclineCategory ---

    #[test]
    fn test_decline_category() {
        assert_eq!(DeclineCategory::from_code("51"), DeclineCategory::InsufficientFunds);
        assert_eq!(DeclineCategory::from_code("54"), DeclineCategory::ExpiredCard);
        assert_eq!(DeclineCategory::from_code("59"), DeclineCategory::SuspectedFraud);
        assert_eq!(DeclineCategory::from_code("99"), DeclineCategory::Other);
    }

    #[test]
    fn test_decline_category_all_codes() {
        assert_eq!(DeclineCategory::from_code("05"), DeclineCategory::DoNotHonor);
        assert_eq!(DeclineCategory::from_code("58"), DeclineCategory::DoNotHonor);
        assert_eq!(DeclineCategory::from_code("51"), DeclineCategory::InsufficientFunds);
        assert_eq!(DeclineCategory::from_code("54"), DeclineCategory::ExpiredCard);
        assert_eq!(DeclineCategory::from_code("36"), DeclineCategory::RestrictedCard);
        assert_eq!(DeclineCategory::from_code("43"), DeclineCategory::RestrictedCard);
        assert_eq!(DeclineCategory::from_code("59"), DeclineCategory::SuspectedFraud);
        assert_eq!(DeclineCategory::from_code("63"), DeclineCategory::SuspectedFraud);
        assert_eq!(DeclineCategory::from_code("65"), DeclineCategory::IncorrectCvc);
        assert_eq!(DeclineCategory::from_code("61"), DeclineCategory::ExceedsLimit);
        assert_eq!(DeclineCategory::from_code("57"), DeclineCategory::MerchantRequest);
        assert_eq!(DeclineCategory::from_code("41"), DeclineCategory::DoNotTryAgain);
        assert_eq!(DeclineCategory::from_code("12"), DeclineCategory::InvalidTransaction);
        assert_eq!(DeclineCategory::from_code("91"), DeclineCategory::ServiceUnavailable);
        assert_eq!(DeclineCategory::from_code("96"), DeclineCategory::ServiceUnavailable);
    }

    #[test]
    fn test_decline_category_unknown_codes() {
        assert_eq!(DeclineCategory::from_code("00"), DeclineCategory::Other);
        assert_eq!(DeclineCategory::from_code("XX"), DeclineCategory::Other);
        assert_eq!(DeclineCategory::from_code(""), DeclineCategory::Other);
    }

    #[test]
    fn test_decline_category_labels() {
        assert_eq!(DeclineCategory::DoNotHonor.label(), "do_not_honor");
        assert_eq!(DeclineCategory::InsufficientFunds.label(), "insufficient_funds");
        assert_eq!(DeclineCategory::ExpiredCard.label(), "expired_card");
        assert_eq!(DeclineCategory::RestrictedCard.label(), "restricted_card");
        assert_eq!(DeclineCategory::SuspectedFraud.label(), "suspected_fraud");
        assert_eq!(DeclineCategory::IncorrectCvc.label(), "incorrect_cvc");
        assert_eq!(DeclineCategory::ExceedsLimit.label(), "exceeds_limit");
        assert_eq!(DeclineCategory::MerchantRequest.label(), "merchant_request");
        assert_eq!(DeclineCategory::DoNotTryAgain.label(), "do_not_try_again");
        assert_eq!(DeclineCategory::InvalidTransaction.label(), "invalid_transaction");
        assert_eq!(DeclineCategory::LimitExceeded.label(), "limit_exceeded");
        assert_eq!(DeclineCategory::ServiceUnavailable.label(), "service_unavailable");
        assert_eq!(DeclineCategory::Other.label(), "other");
    }

    // --- CurrencyAmount ---

    #[test]
    fn test_currency_amount() {
        let amt = CurrencyAmount::new(1500, "usd").unwrap();
        assert_eq!(amt.as_major_units(), 15.0);
        assert!(CurrencyAmount::new(-1, "usd").is_err());
        assert!(CurrencyAmount::new(100, "us").is_err());
    }

    #[test]
    fn test_currency_amount_uppercases() {
        let amt = CurrencyAmount::new(100, "eur").unwrap();
        assert_eq!(amt.currency, "EUR");
    }

    #[test]
    fn test_currency_amount_zero() {
        let amt = CurrencyAmount::new(0, "USD").unwrap();
        assert_eq!(amt.as_major_units(), 0.0);
    }

    #[test]
    fn test_currency_amount_negative_fails() {
        assert!(CurrencyAmount::new(-500, "USD").is_err());
    }

    #[test]
    fn test_currency_amount_wrong_length_currency() {
        assert!(CurrencyAmount::new(100, "US").is_err());
        assert!(CurrencyAmount::new(100, "EURO").is_err());
    }

    // --- AnalyticsMetric ---

    #[test]
    fn test_analytics_metric_labels() {
        assert_eq!(AnalyticsMetric::Volume.label(), "volume");
        assert_eq!(AnalyticsMetric::SuccessRate.label(), "success_rate");
        assert_eq!(AnalyticsMetric::Latency.label(), "latency");
        assert_eq!(AnalyticsMetric::Revenue.label(), "revenue");
        assert_eq!(AnalyticsMetric::DeclineRate.label(), "decline_rate");
        assert_eq!(AnalyticsMetric::AverageTransactionValue.label(), "avg_transaction_value");
        assert_eq!(AnalyticsMetric::RefundRate.label(), "refund_rate");
    }
}
