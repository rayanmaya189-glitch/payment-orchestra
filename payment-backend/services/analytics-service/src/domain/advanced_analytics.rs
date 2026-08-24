//! Advanced Analytics module for real-time metrics, funnel analysis,
//! cohort analysis, and predictive analytics.
//!
//! This module provides:
//! - Real-time transaction metrics with sub-second latency
//! - Conversion funnel analysis
//! - Customer cohort analysis
//! - Predictive churn and revenue forecasting
//! - Custom metric definitions
//! - A/B testing analytics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Real-time Metrics
// ---------------------------------------------------------------------------

/// Real-time metric snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMetrics {
    pub timestamp: DateTime<Utc>,
    pub transactions_per_second: f64,
    pub active_transactions: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub total_volume_minor_units: i64,
    pub total_fees_minor_units: i64,
    pub active_gateways: u32,
    pub error_rate: f64,
}

/// Time series data point for metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Time series with multiple metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub metric_name: String,
    pub unit: String,
    pub data_points: Vec<TimeSeriesPoint>,
    pub aggregation: AggregationType,
}

/// Aggregation type for time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationType {
    Sum,
    Average,
    Count,
    Min,
    Max,
    P50,
    P95,
    P99,
}

// ---------------------------------------------------------------------------
// Funnel Analysis
// ---------------------------------------------------------------------------

/// Funnel step definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStep {
    pub step_name: String,
    pub event_type: String,
    pub filter: Option<std::collections::HashMap<String, String>>,
}

/// Funnel analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelAnalysis {
    pub funnel_id: Uuid,
    pub funnel_name: String,
    pub steps: Vec<FunnelStep>,
    pub results: Vec<FunnelStepResult>,
    pub overall_conversion_rate: f64,
    pub total_entries: u64,
    pub total_completions: u64,
    pub average_time_to_complete_ms: Option<u64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Result for a single funnel step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStepResult {
    pub step_index: usize,
    pub step_name: String,
    pub count: u64,
    pub conversion_rate: f64,
    pub drop_off_rate: f64,
    pub avg_time_to_next_step_ms: Option<u64>,
}

/// Funnel definition for configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelDefinition {
    pub funnel_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<FunnelStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Cohort Analysis
// ---------------------------------------------------------------------------

/// Cohort type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CohortType {
    /// Users who signed up in the same period
    Acquisition,
    /// Users who made their first transaction in the same period
    Transaction,
    /// Users with the same subscription plan
    Subscription,
    /// Custom cohort definition
    Custom,
}

/// Cohort analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortAnalysis {
    pub cohort_id: Uuid,
    pub cohort_name: String,
    pub cohort_type: CohortType,
    pub cohorts: Vec<CohortRow>,
    pub metrics: CohortMetrics,
}

/// A single cohort row in the analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortRow {
    pub cohort_period: String,
    pub cohort_size: u64,
    pub retention_periods: Vec<RetentionData>,
}

/// Retention data for a specific period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionData {
    pub period_index: u32,
    pub active_users: u64,
    pub retention_rate: f64,
    pub revenue: Option<i64>,
}

/// Aggregate metrics for cohort analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortMetrics {
    pub avg_retention_rate: f64,
    pub avg_lifetime_value: f64,
    pub best_cohort: String,
    pub worst_cohort: String,
    pub trend: CohortTrend,
}

/// Cohort trend direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CohortTrend {
    Improving,
    Stable,
    Declining,
}

// ---------------------------------------------------------------------------
// Predictive Analytics
// ---------------------------------------------------------------------------

/// Churn prediction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnPrediction {
    pub operator_id: Uuid,
    pub churn_probability: f64,
    pub risk_factors: Vec<RiskFactor>,
    pub recommended_actions: Vec<String>,
    pub predicted_churn_date: Option<DateTime<Utc>>,
    pub confidence_score: f64,
}

/// Risk factor contributing to churn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_name: String,
    pub impact_score: f64,
    pub description: String,
}

/// Revenue forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueForecast {
    pub forecast_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub predicted_revenue: i64,
    pub confidence_interval: (i64, i64),
    pub prediction_accuracy: f64,
    pub factors: Vec<ForecastFactor>,
    pub historical_trend: Vec<TimeSeriesPoint>,
}

/// Factor influencing revenue forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastFactor {
    pub factor_name: String,
    pub impact: f64,
    pub direction: ForecastDirection,
}

/// Forecast direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForecastDirection {
    Positive,
    Negative,
    Neutral,
}

// ---------------------------------------------------------------------------
// Custom Metrics
// ---------------------------------------------------------------------------

/// Custom metric definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetric {
    pub metric_id: Uuid,
    pub name: String,
    pub description: String,
    pub metric_type: CustomMetricType,
    pub formula: MetricFormula,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Custom metric type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustomMetricType {
    Counter,
    Gauge,
    Histogram,
    Rate,
}

/// Formula for calculating a custom metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricFormula {
    /// Direct field reference
    Field(String),
    /// Arithmetic operation
    Arithmetic {
        left: Box<MetricFormula>,
        operator: ArithmeticOp,
        right: Box<MetricFormula>,
    },
    /// Aggregation over time window
    Aggregate {
        metric: Box<MetricFormula>,
        aggregation: AggregationType,
        window_seconds: u64,
    },
    /// Conditional metric
    Conditional {
        condition: String,
        then: Box<MetricFormula>,
        otherwise: Box<MetricFormula>,
    },
}

/// Arithmetic operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

// ---------------------------------------------------------------------------
// A/B Testing Analytics
// ---------------------------------------------------------------------------

/// A/B test definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbTest {
    pub test_id: Uuid,
    pub name: String,
    pub description: String,
    pub variants: Vec<AbVariant>,
    pub primary_metric: String,
    pub secondary_metrics: Vec<String>,
    pub status: AbTestStatus,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// A/B test variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbVariant {
    pub variant_id: Uuid,
    pub name: String,
    pub description: String,
    pub traffic_percentage: f64,
    pub is_control: bool,
}

/// A/B test status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbTestStatus {
    Draft,
    Running,
    Paused,
    Completed,
}

/// A/B test results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbTestResults {
    pub test_id: Uuid,
    pub status: AbTestStatus,
    pub variants: Vec<AbVariantResults>,
    pub winner: Option<Uuid>,
    pub statistical_significance: f64,
    pub confidence_level: f64,
}

/// Results for a single variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbVariantResults {
    pub variant_id: Uuid,
    pub name: String,
    pub sample_size: u64,
    pub conversions: u64,
    pub conversion_rate: f64,
    pub revenue_per_user: f64,
    pub confidence_interval: (f64, f64),
}

// ---------------------------------------------------------------------------
// Query Types
// ---------------------------------------------------------------------------

/// Query for real-time metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMetricsQuery {
    pub operator_id: Option<Uuid>,
    pub metrics: Vec<String>,
}

/// Query for time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesQuery {
    pub metric_name: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub granularity: TimeGranularity,
    pub filters: Option<std::collections::HashMap<String, String>>,
}

/// Time granularity for aggregations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeGranularity {
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

/// Query for funnel analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelQuery {
    pub funnel_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub segment: Option<std::collections::HashMap<String, String>>,
}

/// Query for cohort analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortQuery {
    pub cohort_type: CohortType,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub period_days: u32,
}

/// Query for churn prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnPredictionQuery {
    pub operator_ids: Option<Vec<Uuid>>,
    pub risk_threshold: f64,
}

/// Query for revenue forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueForecastQuery {
    pub forecast_days: u32,
    pub include_confidence_interval: bool,
    pub segments: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Analytics Service Trait
// ---------------------------------------------------------------------------

/// Advanced analytics service trait.
#[async_trait::async_trait]
pub trait AdvancedAnalyticsService: Send + Sync {
    /// Get real-time metrics snapshot.
    async fn get_realtime_metrics(
        &self,
        query: RealtimeMetricsQuery,
    ) -> Result<RealtimeMetrics, super::AnalyticsError>;

    /// Get time series data.
    async fn get_time_series(
        &self,
        query: TimeSeriesQuery,
    ) -> Result<TimeSeries, super::AnalyticsError>;

    /// Run funnel analysis.
    async fn analyze_funnel(
        &self,
        query: FunnelQuery,
    ) -> Result<FunnelAnalysis, super::AnalyticsError>;

    /// Run cohort analysis.
    async fn analyze_cohort(
        &self,
        query: CohortQuery,
    ) -> Result<CohortAnalysis, super::AnalyticsError>;

    /// Get churn predictions.
    async fn predict_churn(
        &self,
        query: ChurnPredictionQuery,
    ) -> Result<Vec<ChurnPrediction>, super::AnalyticsError>;

    /// Get revenue forecast.
    async fn forecast_revenue(
        &self,
        query: RevenueForecastQuery,
    ) -> Result<RevenueForecast, super::AnalyticsError>;

    /// Create custom metric.
    async fn create_custom_metric(
        &self,
        metric: CustomMetric,
    ) -> Result<CustomMetric, super::AnalyticsError>;

    /// Get custom metric value.
    async fn get_custom_metric_value(
        &self,
        metric_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<f64, super::AnalyticsError>;

    /// Create A/B test.
    async fn create_ab_test(
        &self,
        test: AbTest,
    ) -> Result<AbTest, super::AnalyticsError>;

    /// Get A/B test results.
    async fn get_ab_test_results(
        &self,
        test_id: Uuid,
    ) -> Result<AbTestResults, super::AnalyticsError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_realtime_metrics_serialization() {
        let metrics = RealtimeMetrics {
            timestamp: Utc::now(),
            transactions_per_second: 1234.5,
            active_transactions: 5678,
            success_rate: 98.5,
            avg_latency_ms: 142.0,
            p95_latency_ms: 350.0,
            p99_latency_ms: 890.0,
            total_volume_minor_units: 123456789,
            total_fees_minor_units: 1234567,
            active_gateways: 5,
            error_rate: 1.5,
        };

        let json = serde_json::to_string(&metrics).unwrap();
        let parsed: RealtimeMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.transactions_per_second, 1234.5);
        assert_eq!(parsed.active_transactions, 5678);
    }

    #[test]
    fn test_funnel_analysis() {
        let analysis = FunnelAnalysis {
            funnel_id: Uuid::now_v7(),
            funnel_name: "Checkout Funnel".to_string(),
            steps: vec![],
            results: vec![
                FunnelStepResult {
                    step_index: 0,
                    step_name: "Page View".to_string(),
                    count: 10000,
                    conversion_rate: 100.0,
                    drop_off_rate: 0.0,
                    avg_time_to_next_step_ms: Some(5000),
                },
                FunnelStepResult {
                    step_index: 1,
                    step_name: "Add to Cart".to_string(),
                    count: 3500,
                    conversion_rate: 35.0,
                    drop_off_rate: 65.0,
                    avg_time_to_next_step_ms: Some(120000),
                },
                FunnelStepResult {
                    step_index: 2,
                    step_name: "Checkout".to_string(),
                    count: 1200,
                    conversion_rate: 12.0,
                    drop_off_rate: 88.0,
                    avg_time_to_next_step_ms: Some(60000),
                },
            ],
            overall_conversion_rate: 12.0,
            total_entries: 10000,
            total_completions: 1200,
            average_time_to_complete_ms: Some(185000),
            period_start: Utc::now() - Duration::days(30),
            period_end: Utc::now(),
        };

        assert_eq!(analysis.results.len(), 3);
        assert_eq!(analysis.overall_conversion_rate, 12.0);
    }
}
