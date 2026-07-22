//! Domain event definitions for BC-01 Operator Management.
//! Events are published via in-process NATS channels for downstream consumers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Events emitted by operator-service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperatorEvent {
    Registered(OperatorRegistered),
    Verified(OperatorVerified),
    Suspended(OperatorSuspended),
    Reactivated(OperatorReactivated),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRegistered {
    pub operator_id: Uuid,
    pub legal_name: String,
    pub email: String,
    pub subdomain: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorVerified {
    pub operator_id: Uuid,
    pub previous_status: String,
    pub new_status: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorSuspended {
    pub operator_id: Uuid,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorReactivated {
    pub operator_id: Uuid,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

impl OperatorEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Registered(_) => "operator_registered",
            Self::Verified(_) => "operator_verified",
            Self::Suspended(_) => "operator_suspended",
            Self::Reactivated(_) => "operator_reactivated",
        }
    }

    #[allow(dead_code)]
    pub fn operator_id(&self) -> Uuid {
        match self {
            Self::Registered(e) => e.operator_id,
            Self::Verified(e) => e.operator_id,
            Self::Suspended(e) => e.operator_id,
            Self::Reactivated(e) => e.operator_id,
        }
    }
}
