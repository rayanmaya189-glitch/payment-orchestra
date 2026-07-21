use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct PaymentAnalytics {
    pub period_start: DateTime<Utc>, pub period_end: DateTime<Utc>,
    pub total_volume: i64, pub total_count: i64,
    pub success_count: i64, pub failure_count: i64,
    pub success_rate: f64, pub avg_latency_ms: f64,
    pub revenue: i64, pub refund_amount: i64,
}

impl PaymentAnalytics {
    pub fn new(period_start: DateTime<Utc>, period_end: DateTime<Utc>) -> Self {
        Self { period_start, period_end, total_volume: 0, total_count: 0, success_count: 0, failure_count: 0, success_rate: 0.0, avg_latency_ms: 0.0, revenue: 0, refund_amount: 0 }
    }

    pub fn calculate_rates(&mut self) {
        self.success_rate = if self.total_count > 0 { (self.success_count as f64 / self.total_count as f64) * 100.0 } else { 0.0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_analytics() {
        let mut a = PaymentAnalytics::new(Utc::now() - chrono::Duration::hours(24), Utc::now());
        a.total_count = 100;
        a.success_count = 95;
        a.failure_count = 5;
        a.calculate_rates();
        assert_eq!(a.success_rate, 95.0);
    }
}
