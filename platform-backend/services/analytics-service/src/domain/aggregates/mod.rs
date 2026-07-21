use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::value_objects::TimeRange;

#[derive(Debug, Clone)]
pub struct PaymentAnalytics {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_volume: i64,
    pub total_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub revenue: i64,
    pub refund_amount: i64,
}

impl PaymentAnalytics {
    pub fn new(period_start: DateTime<Utc>, period_end: DateTime<Utc>) -> Self {
        Self {
            period_start,
            period_end,
            total_volume: 0,
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            success_rate: 0.0,
            avg_latency_ms: 0.0,
            revenue: 0,
            refund_amount: 0,
        }
    }

    pub fn from_time_range(time_range: &TimeRange) -> Self {
        Self::new(time_range.start, time_range.end)
    }

    pub fn calculate_rates(&mut self) {
        self.success_rate = if self.total_count > 0 {
            (self.success_count as f64 / self.total_count as f64) * 100.0
        } else {
            0.0
        };
    }

    pub fn calculate_avg_latency(&mut self, latencies: &[u32]) {
        if latencies.is_empty() {
            self.avg_latency_ms = 0.0;
            return;
        }
        let sum: u64 = latencies.iter().map(|&l| l as u64).sum();
        self.avg_latency_ms = sum as f64 / latencies.len() as f64;
    }

    pub fn decline_rate(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        (self.failure_count as f64 / self.total_count as f64) * 100.0
    }

    pub fn average_transaction_value(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.total_volume as f64 / self.total_count as f64
    }

    pub fn refund_rate(&self) -> f64 {
        if self.revenue == 0 {
            return 0.0;
        }
        (self.refund_amount as f64 / self.revenue as f64) * 100.0
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "period_start": self.period_start.to_rfc3339(),
            "period_end": self.period_end.to_rfc3339(),
            "total_volume": self.total_volume,
            "total_count": self.total_count,
            "success_count": self.success_count,
            "failure_count": self.failure_count,
            "success_rate": self.success_rate,
            "avg_latency_ms": self.avg_latency_ms,
            "revenue": self.revenue,
            "refund_amount": self.refund_amount,
            "decline_rate": self.decline_rate(),
            "avg_transaction_value": self.average_transaction_value(),
            "refund_rate": self.refund_rate(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_creation() {
        let mut a = PaymentAnalytics::new(
            Utc::now() - chrono::Duration::hours(24),
            Utc::now(),
        );
        a.total_count = 100;
        a.success_count = 95;
        a.failure_count = 5;
        a.calculate_rates();
        assert_eq!(a.success_rate, 95.0);
        assert!((a.decline_rate() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_analytics_empty() {
        let a = PaymentAnalytics::new(Utc::now() - chrono::Duration::hours(24), Utc::now());
        assert_eq!(a.decline_rate(), 0.0);
        assert_eq!(a.average_transaction_value(), 0.0);
        assert_eq!(a.refund_rate(), 0.0);
    }

    #[test]
    fn test_analytics_with_volume() {
        let mut a = PaymentAnalytics::new(Utc::now() - chrono::Duration::hours(24), Utc::now());
        a.total_count = 10;
        a.total_volume = 10000;
        assert!((a.average_transaction_value() - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_analytics_refund_rate() {
        let mut a = PaymentAnalytics::new(Utc::now() - chrono::Duration::hours(24), Utc::now());
        a.revenue = 1000;
        a.refund_amount = 50;
        assert!((a.refund_rate() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_analytics_avg_latency() {
        let mut a = PaymentAnalytics::new(Utc::now() - chrono::Duration::hours(24), Utc::now());
        a.calculate_avg_latency(&[100, 200, 300]);
        assert!((a.avg_latency_ms - 200.0).abs() < f64::EPSILON);
    }
}
