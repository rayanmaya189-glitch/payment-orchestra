//! AI Gateway events

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiGatewayEvent {
    QueryProcessed(QueryProcessedPayload),
    QueryBlocked(QueryBlockedPayload),
    CircuitBreakerTripped(CircuitPayload),
    QuotaExceeded(QuotaPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProcessedPayload {
    pub query_id: Uuid,
    pub operator_id: Uuid,
    pub allowed: bool,
    pub route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryBlockedPayload {
    pub query_id: Uuid,
    pub operator_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitPayload {
    pub failure_count: u32,
    pub threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaPayload {
    pub operator_id: Uuid,
    pub queries_limit: u32,
}
