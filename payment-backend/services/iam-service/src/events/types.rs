//! Domain event definitions for BC-02 Identity & Access Management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IamEvent {
    PrincipalCreated(PrincipalCreated),
    PrincipalAuthenticated(PrincipalAuthenticated),
    PermissionDenied(PermissionDenied),
    ApiKeyCreated(ApiKeyCreated),
    ApiKeyRevoked(ApiKeyRevoked),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalCreated {
    pub principal_id: Uuid,
    pub email: String,
    pub principal_type: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalAuthenticated {
    pub principal_id: Uuid,
    pub ip_address: String,
    pub user_agent: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDenied {
    pub principal_id: Uuid,
    pub resource: String,
    pub action: String,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCreated {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub scopes: Vec<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRevoked {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

impl IamEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::PrincipalCreated(_) => "principal_created",
            Self::PrincipalAuthenticated(_) => "principal_authenticated",
            Self::PermissionDenied(_) => "permission_denied",
            Self::ApiKeyCreated(_) => "api_key_created",
            Self::ApiKeyRevoked(_) => "api_key_revoked",
        }
    }
}
