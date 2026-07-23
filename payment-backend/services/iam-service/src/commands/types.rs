//! Command types for BC-02 Identity & Access Management.

use uuid::Uuid;

use crate::domain::{ApiKey, PendingChange, Principal};

// ─── Command Types ──────────────────────────────────────────────────────────

pub struct Authenticate {
    pub email: String,
    pub password: String,
    pub ip_address: std::net::IpAddr,
    pub user_agent: String,
}

pub struct CreateApiKey {
    pub principal_id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

pub struct RevokeApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
}

pub struct SubmitChange {
    pub change_type: String,
    pub maker_id: Uuid,
    pub payload: Vec<u8>,
    pub maker_note: Option<String>,
}

pub struct ReviewChange {
    pub change_id: Uuid,
    pub checker_id: Uuid,
    pub approved: bool,
    pub checker_note: Option<String>,
}

// ─── Results ────────────────────────────────────────────────────────────────

pub struct AuthenticateResult {
    pub principal: Principal,
    pub access_token: String,
    pub refresh_token: String,
    pub mfa_required: bool,
    pub mfa_method: Option<String>,
}

pub struct CreateApiKeyResult {
    pub api_key: ApiKey,
    pub api_key_secret: String,
}

pub struct RevokeApiKeyResult {
    pub revoked: bool,
}

pub struct SubmitChangeResult {
    pub change: PendingChange,
}

pub struct ReviewChangeResult {
    pub change: PendingChange,
}
