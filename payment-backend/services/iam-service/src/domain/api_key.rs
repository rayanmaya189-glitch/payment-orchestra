use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An API key associated with a principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub name: String,
    pub key_hash: Vec<u8>,
    pub scopes: Vec<String>,
    pub status: ApiKeyStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

impl ApiKeyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "revoked" => Some(Self::Revoked),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}
