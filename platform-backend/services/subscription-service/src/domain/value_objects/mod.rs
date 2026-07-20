use chrono::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionStatus {
    Active,
    Trialing,
    PastDue,
    Canceled,
    Unpaid,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Trialing => "trialing",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Unpaid => "unpaid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "trialing" => Self::Trialing,
            "past_due" => Self::PastDue,
            "canceled" => Self::Canceled,
            "unpaid" => Self::Unpaid,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionInterval {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl SubscriptionInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
            Self::Yearly => "yearly",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "daily" => Self::Daily,
            "weekly" => Self::Weekly,
            "quarterly" => Self::Quarterly,
            "yearly" => Self::Yearly,
            _ => Self::Monthly,
        }
    }

    pub fn duration(&self, count: i32) -> Duration {
        match self {
            Self::Daily => Duration::days(count as i64),
            Self::Weekly => Duration::weeks(count as i64),
            Self::Monthly => Duration::days(count as i64 * 30),
            Self::Quarterly => Duration::days(count as i64 * 90),
            Self::Yearly => Duration::days(count as i64 * 365),
        }
    }
}
