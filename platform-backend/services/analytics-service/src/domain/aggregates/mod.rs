use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{MetricType, TimeGranularity};

/// Analytics dashboard widget configuration.
#[derive(Debug, Clone)]
pub struct DashboardWidget {
    pub widget_id: Uuid,
    pub operator_id: Uuid,
    pub widget_type: MetricType,
    pub title: String,
    pub granularity: TimeGranularity,
    pub filters: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl DashboardWidget {
    pub fn new(operator_id: Uuid, widget_type: MetricType, title: String, granularity: TimeGranularity) -> Self {
        Self {
            widget_id: Uuid::now_v7(),
            operator_id,
            widget_type,
            title,
            granularity,
            filters: None,
            created_at: Utc::now(),
        }
    }
}

/// Analytics data point — a single metric value.
#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub label: Option<String>,
}

/// Analytics query result — aggregated metrics.
#[derive(Debug, Clone)]
pub struct AnalyticsResult {
    pub query_id: Uuid,
    pub widget_id: Uuid,
    pub data_points: Vec<MetricDataPoint>,
    pub total: Option<f64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl AnalyticsResult {
    pub fn new(widget_id: Uuid, period_start: DateTime<Utc>, period_end: DateTime<Utc>) -> Self {
        Self {
            query_id: Uuid::now_v7(),
            widget_id,
            data_points: Vec::new(),
            total: None,
            period_start,
            period_end,
        }
    }

    pub fn calculate_total(&mut self) {
        self.total = Some(self.data_points.iter().map(|dp| dp.value).sum());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_widget() {
        let w = DashboardWidget::new(Uuid::now_v7(), MetricType::AuthorizationRate, "Auth Rate".into(), TimeGranularity::Hourly);
        assert_eq!(w.title, "Auth Rate");
    }

    #[test]
    fn test_analytics_result() {
        let mut r = AnalyticsResult::new(Uuid::now_v7(), Utc::now() - chrono::Duration::hours(24), Utc::now());
        r.data_points.push(MetricDataPoint { timestamp: Utc::now(), value: 100.0, label: None });
        r.data_points.push(MetricDataPoint { timestamp: Utc::now(), value: 200.0, label: None });
        r.calculate_total();
        assert_eq!(r.total, Some(300.0));
    }
}
