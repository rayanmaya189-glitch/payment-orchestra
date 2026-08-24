//! API Gateway events

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GatewayEvent {
    RequestProcessed(RequestPayload),
    RequestBlocked(BlockedPayload),
    AuthenticationFailed(AuthPayload),
    RateLimitTriggered(RateLimitPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPayload {
    pub request_id: Uuid,
    pub path: String,
    pub method: String,
    pub status: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedPayload {
    pub request_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPayload {
    pub request_id: Uuid,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitPayload {
    pub request_id: Uuid,
    pub scope: String,
    pub limit: u32,
}
