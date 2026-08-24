//! Command types for BYOK Core — MerchantAcquirerLink lifecycle management.

use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::{MerchantAcquirerLink, LinkEnvironment};

// ─── Command Types ──────────────────────────────────────────────────────────

pub struct CreateLink {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub display_name: String,
    pub environment: LinkEnvironment,
    pub credentials: HashMap<String, String>,
}

pub struct TestConnection {
    pub link_id: Uuid,
}

pub struct RotateCredentials {
    pub link_id: Uuid,
    pub new_credentials: HashMap<String, String>,
    pub rotate_immediately: bool,
}

pub struct DisableLink {
    pub link_id: Uuid,
    pub reason: String,
}

pub struct EnableLink {
    pub link_id: Uuid,
}

pub struct UpdateMetadata {
    pub link_id: Uuid,
    pub display_name: Option<String>,
}

// ─── Results ────────────────────────────────────────────────────────────────

pub struct CreateLinkResult {
    pub link: MerchantAcquirerLink,
}

pub struct TestConnectionResult {
    pub link: MerchantAcquirerLink,
    pub success: bool,
    pub latency_ms: u32,
    pub error_message: Option<String>,
}

pub struct RotateCredentialsResult {
    pub link: MerchantAcquirerLink,
    pub old_credentials_retained: bool,
}

pub struct DisableLinkResult {
    pub link: MerchantAcquirerLink,
}

pub struct EnableLinkResult {
    pub link: MerchantAcquirerLink,
}

pub struct UpdateMetadataResult {
    pub link: MerchantAcquirerLink,
}
