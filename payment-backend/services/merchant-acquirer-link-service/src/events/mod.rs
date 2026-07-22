//! Domain event definitions for BYOK Core — MerchantAcquirerLink lifecycle.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum LinkEvent {
    Created(LinkCreated),
    Enabled(LinkEnabled),
    Disabled(LinkDisabled),
    CredentialsRotated(CredentialsRotated),
    ConnectionTested(ConnectionTested),
    CredentialsExpiring(CredentialsExpiring),
    CredentialsExpired(CredentialsExpired),
    HealthChanged(HealthChanged),
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkCreated {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub environment: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkEnabled {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkDisabled {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialsRotated {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub rotated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionTested {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub success: bool,
    pub latency_ms: u32,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialsExpiring {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub days_until_expiry: u32,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialsExpired {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthChanged {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub old_health: String,
    pub new_health: String,
    pub occurred_at: DateTime<Utc>,
}

impl LinkEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "merchant_acquirer_link_created",
            Self::Enabled(_) => "merchant_acquirer_link_enabled",
            Self::Disabled(_) => "merchant_acquirer_link_disabled",
            Self::CredentialsRotated(_) => "merchant_acquirer_credentials_rotated",
            Self::ConnectionTested(_) => "merchant_acquirer_connection_tested",
            Self::CredentialsExpiring(_) => "merchant_acquirer_credentials_expiring",
            Self::CredentialsExpired(_) => "merchant_acquirer_credentials_expired",
            Self::HealthChanged(_) => "merchant_acquirer_link_health_changed",
        }
    }
}
