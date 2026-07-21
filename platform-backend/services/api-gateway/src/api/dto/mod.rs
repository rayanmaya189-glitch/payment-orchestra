use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RouteRequestDto {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ForwardRequestDto {
    pub method: String,
    pub path: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
    pub principal_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRouteDto {
    pub path_prefix: String,
    pub target_service: String,
    pub target_url: String,
    pub auth_required: Option<bool>,
    pub rate_limit_max_requests: Option<u32>,
    pub rate_limit_window_seconds: Option<u32>,
    pub methods: Option<Vec<String>>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRouteDto {
    pub path_prefix: Option<String>,
    pub target_service: Option<String>,
    pub target_url: Option<String>,
    pub auth_required: Option<bool>,
    pub rate_limit_max_requests: Option<u32>,
    pub rate_limit_window_seconds: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct RouteDto {
    pub path_prefix: String,
    pub target_service: String,
    pub target_url: String,
    pub auth_required: bool,
    pub rate_limit: Option<RateLimitDto>,
    pub methods: Vec<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitDto {
    pub max_requests: u32,
    pub window_seconds: u32,
}

#[derive(Debug, Serialize)]
pub struct ForwardResultDto {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub target_service: String,
    pub status: String,
    pub response_status: Option<u16>,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GatewayStatsDto {
    pub total_requests: u64,
    pub successful_forwards: u64,
    pub failed_forwards: u64,
    pub rate_limited: u64,
    pub not_found: u64,
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
    pub active_routes: u64,
}

#[derive(Debug, Serialize)]
pub struct ErrorDto {
    pub error: String,
    pub code: String,
}
