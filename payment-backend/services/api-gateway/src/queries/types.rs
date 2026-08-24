//! Query types for API Gateway

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayHealth {
    pub is_healthy: bool,
    pub routes_count: usize,
    pub uptime_hours: u64,
    pub message: String,
}
