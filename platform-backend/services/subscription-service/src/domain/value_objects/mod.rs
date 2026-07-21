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

    pub fn can_transition_to(&self, target: &SubscriptionStatus) -> bool {
        matches!(
            (self, target),
            (Self::Active, Self::PastDue)
                | (Self::Active, Self::Canceled)
                | (Self::Trialing, Self::Active)
                | (Self::Trialing, Self::PastDue)
                | (Self::Trialing, Self::Canceled)
                | (Self::PastDue, Self::Active)
                | (Self::PastDue, Self::Canceled)
                | (Self::PastDue, Self::Unpaid)
                | (Self::Canceled, Self::Active)
                | (Self::Unpaid, Self::Canceled)
        )
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

/// Dunning configuration for retry scheduling after payment failure.
#[derive(Debug, Clone)]
pub struct DunningConfig {
    pub max_retries: i32,
    pub retry_intervals: Vec<Duration>,
}

impl DunningConfig {
    /// Standard dunning schedule: retry at 1 day, 3 days, 7 days, 14 days, 30 days.
    pub fn standard() -> Self {
        Self {
            max_retries: 5,
            retry_intervals: vec![
                Duration::days(1),
                Duration::days(3),
                Duration::days(7),
                Duration::days(14),
                Duration::days(30),
            ],
        }
    }

    /// Aggressive dunning: retry daily for 5 days.
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            retry_intervals: vec![
                Duration::days(1),
                Duration::days(1),
                Duration::days(1),
                Duration::days(1),
                Duration::days(1),
            ],
        }
    }

    /// Lenient dunning: retry at 3 days, 7 days, 14 days.
    pub fn lenient() -> Self {
        Self {
            max_retries: 3,
            retry_intervals: vec![
                Duration::days(3),
                Duration::days(7),
                Duration::days(14),
            ],
        }
    }

    /// Get the delay for a given retry attempt (0-indexed).
    /// Returns the last interval if retry exceeds configured intervals.
    pub fn delay_for_attempt(&self, attempt: i32) -> Duration {
        let idx = (attempt as usize).min(self.retry_intervals.len().saturating_sub(1));
        self.retry_intervals
            .get(idx)
            .copied()
            .unwrap_or_else(|| Duration::days(1))
    }

    pub fn from_profile(profile: &str) -> Self {
        match profile {
            "aggressive" => Self::aggressive(),
            "lenient" => Self::lenient(),
            _ => Self::standard(),
        }
    }
}

impl Default for DunningConfig {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transitions() {
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::PastDue));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Canceled));
        assert!(!SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Unpaid));
        assert!(!SubscriptionStatus::Canceled.can_transition_to(&SubscriptionStatus::PastDue));
        assert!(SubscriptionStatus::Canceled.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::PastDue.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::PastDue.can_transition_to(&SubscriptionStatus::Unpaid));
        assert!(SubscriptionStatus::Unpaid.can_transition_to(&SubscriptionStatus::Canceled));
    }

    #[test]
    fn test_dunning_delay() {
        let config = DunningConfig::standard();
        assert_eq!(config.delay_for_attempt(0), Duration::days(1));
        assert_eq!(config.delay_for_attempt(1), Duration::days(3));
        assert_eq!(config.delay_for_attempt(2), Duration::days(7));
        assert_eq!(config.delay_for_attempt(3), Duration::days(14));
        assert_eq!(config.delay_for_attempt(4), Duration::days(30));
        // Beyond configured intervals, uses last
        assert_eq!(config.delay_for_attempt(10), Duration::days(30));
    }

    #[test]
    fn test_dunning_profiles() {
        let aggressive = DunningConfig::aggressive();
        assert_eq!(aggressive.max_retries, 5);
        assert_eq!(aggressive.retry_intervals[0], Duration::days(1));

        let lenient = DunningConfig::lenient();
        assert_eq!(lenient.max_retries, 3);
        assert_eq!(lenient.retry_intervals[0], Duration::days(3));

        let from_profile = DunningConfig::from_profile("aggressive");
        assert_eq!(from_profile.max_retries, aggressive.max_retries);
    }

    #[test]
    fn test_interval_duration() {
        assert_eq!(
            SubscriptionInterval::Monthly.duration(1),
            Duration::days(30)
        );
        assert_eq!(
            SubscriptionInterval::Yearly.duration(1),
            Duration::days(365)
        );
        assert_eq!(
            SubscriptionInterval::Daily.duration(7),
            Duration::days(7)
        );
    }
}
