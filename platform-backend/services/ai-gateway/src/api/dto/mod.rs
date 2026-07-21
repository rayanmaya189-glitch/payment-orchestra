use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AiRequestDto {
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct AiResponseDto {
    pub request_id: String,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub block_category: Option<String>,
    pub redacted_prompt: Option<String>,
    pub model: String,
    pub tokens_used: Option<u32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<u64>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetRequestQuery {
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ListRequestsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UsageStatsDto {
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub block_rate: f64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: f64,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorDto {
    pub error: String,
    pub code: String,
}
