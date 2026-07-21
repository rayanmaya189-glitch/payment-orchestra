use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::value_objects::{CurrencyAmount, DeclineCategory};

#[derive(Debug, Clone)]
pub struct AuthorizationEvent {
    pub event_id: Uuid,
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub status: AuthorizationStatus,
    pub amount: CurrencyAmount,
    pub decline_code: Option<String>,
    pub decline_category: Option<DeclineCategory>,
    pub latency_ms: u32,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationStatus {
    Approved,
    Declined,
    Pending,
    Error,
    Timeout,
}

impl AuthorizationStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Approved => "approved",
            Self::Declined => "declined",
            Self::Pending => "pending",
            Self::Error => "error",
            Self::Timeout => "timeout",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeclineBreakdownEntry {
    pub category: DeclineCategory,
    pub count: i64,
    pub percentage: f64,
    pub top_connectors: Vec<ConnectorDecline>,
}

#[derive(Debug, Clone)]
pub struct ConnectorDecline {
    pub connector_id: String,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct VolumeBucket {
    pub timestamp: DateTime<Utc>,
    pub transaction_count: i64,
    pub volume_minor_units: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone)]
pub struct ConnectorPerformance {
    pub connector_id: String,
    pub total_requests: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub total_volume: i64,
}

#[derive(Debug, Clone)]
pub struct HourlyTrend {
    pub hour: DateTime<Utc>,
    pub transaction_count: i64,
    pub volume: i64,
    pub success_rate: f64,
}

#[derive(Debug, Clone)]
pub struct OperatorAnalytics {
    pub operator_id: Uuid,
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
    pub decline_breakdown: Vec<DeclineBreakdownEntry>,
    pub hourly_trends: Vec<HourlyTrend>,
    pub connector_performance: Vec<ConnectorPerformance>,
}

impl OperatorAnalytics {
    pub fn new(operator_id: Uuid, period_start: DateTime<Utc>, period_end: DateTime<Utc>) -> Self {
        Self {
            operator_id,
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
            decline_breakdown: Vec::new(),
            hourly_trends: Vec::new(),
            connector_performance: Vec::new(),
        }
    }

    pub fn calculate_rates(&mut self) {
        self.success_rate = if self.total_count > 0 {
            (self.success_count as f64 / self.total_count as f64) * 100.0
        } else {
            0.0
        };
    }

    pub fn to_summary_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operator_id": self.operator_id.to_string(),
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_status() {
        assert_eq!(AuthorizationStatus::Approved.label(), "approved");
        assert_eq!(AuthorizationStatus::Declined.label(), "declined");
    }

    #[test]
    fn test_decline_breakdown_entry() {
        let entry = DeclineBreakdownEntry {
            category: DeclineCategory::InsufficientFunds,
            count: 42,
            percentage: 35.0,
            top_connectors: vec![],
        };
        assert_eq!(entry.category, DeclineCategory::InsufficientFunds);
        assert_eq!(entry.count, 42);
    }

    #[test]
    fn test_operator_analytics() {
        let mut analytics = OperatorAnalytics::new(
            Uuid::now_v7(),
            Utc::now() - chrono::Duration::hours(24),
            Utc::now(),
        );
        analytics.total_count = 100;
        analytics.success_count = 95;
        analytics.failure_count = 5;
        analytics.calculate_rates();
        assert!((analytics.success_rate - 95.0).abs() < f64::EPSILON);
    }
}
