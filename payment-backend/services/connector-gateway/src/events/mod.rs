//! Domain events published by connector-gateway.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum GatewayEvent {
    GatewayProfileCreated(GatewayProfileCreated),
    GatewayProfileUpdated(GatewayProfileUpdated),
    ConnectionTested(ConnectionTested),
    CredentialsValidated(CredentialsValidated),
    CircuitBreakerStateChanged(CircuitBreakerStateChanged),
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayProfileCreated {
    pub profile_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayProfileUpdated {
    pub profile_id: Uuid,
    pub operator_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionTested {
    pub profile_id: Uuid,
    pub success: bool,
    pub latency_ms: u32,
    pub error_message: Option<String>,
    pub tested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialsValidated {
    pub connector_id: String,
    pub valid: bool,
    pub merchant_name: Option<String>,
    pub validated_at: DateTime<Utc>,
}

impl fmt::Display for GatewayEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GatewayEvent::GatewayProfileCreated(e) => write!(f, "GatewayProfileCreated({})", e.profile_id),
            GatewayEvent::GatewayProfileUpdated(e) => write!(f, "GatewayProfileUpdated({})", e.profile_id),
            GatewayEvent::ConnectionTested(e) => write!(f, "ConnectionTested({}, success={})", e.profile_id, e.success),
            GatewayEvent::CredentialsValidated(e) => write!(f, "CredentialsValidated({}, valid={})", e.connector_id, e.valid),
            GatewayEvent::CircuitBreakerStateChanged(e) => write!(f, "CircuitBreakerStateChanged({}: {} -> {})", e.connector_id, e.previous_state, e.new_state),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CircuitBreakerStateChanged {
    pub connector_id: String,
    pub previous_state: String,
    pub new_state: String,
    pub changed_at: DateTime<Utc>,
}
