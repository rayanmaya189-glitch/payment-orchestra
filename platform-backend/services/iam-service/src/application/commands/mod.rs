#![allow(dead_code)]
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthenticateCommand {
    pub email: String,
    pub password: String,
    pub ip_address: IpAddr,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct IssueTokenCommand {
    pub refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct CreateApiKeyCommand {
    pub principal_id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub acquirer_link_ids: Option<Vec<Uuid>>,
    pub expires_in_days: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RevokeApiKeyCommand {
    pub api_key_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RotateApiKeyCommand {
    pub api_key_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ApprovePendingChangeCommand {
    pub change_id: Uuid,
    pub checker_id: Uuid,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InvalidateAllSessionsCommand {
    pub principal_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RevokeAllSessionsCommand {
    pub operator_id: Uuid,
}
