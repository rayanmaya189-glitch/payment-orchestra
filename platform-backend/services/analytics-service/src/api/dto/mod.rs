use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AnalyticsQueryDto {
    pub operator_id: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PaymentAnalyticsDto {
    pub period_start: String,
    pub period_end: String,
    pub total_volume: i64,
    pub total_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub revenue: i64,
    pub refund_amount: i64,
    pub decline_rate: f64,
    pub avg_transaction_value: f64,
}

#[derive(Debug, Serialize)]
pub struct VolumeBucketDto {
    pub timestamp: String,
    pub transaction_count: i64,
    pub volume: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct DeclineEntryDto {
    pub category: String,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct ConnectorPerformanceDto {
    pub connector_id: String,
    pub total_requests: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub total_volume: i64,
}

#[derive(Debug, Serialize)]
pub struct ErrorDto {
    pub error: String,
    pub code: String,
}
