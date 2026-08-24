use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DeepHealth {
    pub service: String,
    pub status: String,
    pub checks: Vec<HealthCheckResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckResult {
    pub name: String,
    pub status: String,
    pub latency_ms: u64,
    pub error: Option<String>,
}
