#[derive(Debug, Clone)]
pub struct TimeRange { pub start: String, pub end: String }
#[derive(Debug, Clone)]
pub enum AnalyticsMetric { Volume, SuccessRate, Latency, Revenue }
