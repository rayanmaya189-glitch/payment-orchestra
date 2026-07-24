//! Domain model for BYOK Core — MerchantAcquirerLink aggregate.
//! Connects an operator to a specific payment gateway using their own credentials.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// MerchantAcquirerLink — represents ONE operator's connection to ONE gateway using ONE set of credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantAcquirerLink {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub display_name: String,
    pub environment: LinkEnvironment,
    pub encrypted_credentials: Vec<u8>,
    pub credentials_hash: String,
    pub status: LinkStatus,
    pub health_status: HealthStatus,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_healthy_at: Option<DateTime<Utc>>,
    pub credentials_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkEnvironment {
    Sandbox,
    Production,
}

impl LinkEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::Production => "production",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sandbox" => Some(Self::Sandbox),
            "production" => Some(Self::Production),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStatus {
    Testing,
    Active,
    Disabled,
    CredentialsExpired,
}

impl LinkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Testing => "testing",
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::CredentialsExpired => "credentials_expired",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "testing" => Some(Self::Testing),
            "active" => Some(Self::Active),
            "disabled" => Some(Self::Disabled),
            "credentials_expired" => Some(Self::CredentialsExpired),
            _ => None,
        }
    }

    pub fn is_routable(&self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unreachable,
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unreachable => "unreachable",
            Self::Unknown => "unknown",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "healthy" => Some(Self::Healthy),
            "degraded" => Some(Self::Degraded),
            "unreachable" => Some(Self::Unreachable),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

impl MerchantAcquirerLink {
    pub fn new(
        operator_id: Uuid,
        connector_id: String,
        display_name: String,
        environment: LinkEnvironment,
        encrypted_credentials: Vec<u8>,
        credentials_hash: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            link_id: Uuid::now_v7(),
            operator_id,
            connector_id,
            display_name,
            environment,
            encrypted_credentials,
            credentials_hash,
            status: LinkStatus::Testing,
            health_status: HealthStatus::Unknown,
            last_tested_at: None,
            last_healthy_at: None,
            credentials_expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Test connection result handler
    pub fn record_connection_test(&mut self, success: bool, _latency_ms: u32) {
        self.last_tested_at = Some(Utc::now());
        if success {
            self.health_status = HealthStatus::Healthy;
            self.last_healthy_at = Some(Utc::now());
            self.status = LinkStatus::Active;
        } else {
            self.health_status = HealthStatus::Unreachable;
        }
        self.updated_at = Utc::now();
    }

    /// Rotate credentials
    pub fn rotate_credentials(&mut self, new_encrypted: Vec<u8>, new_hash: String) {
        self.encrypted_credentials = new_encrypted;
        self.credentials_hash = new_hash;
        self.status = LinkStatus::Testing; // must test after rotation
        self.health_status = HealthStatus::Unknown;
        self.updated_at = Utc::now();
    }

    /// Disable this link
    pub fn disable(&mut self) -> Result<(), LinkError> {
        if self.status == LinkStatus::Disabled {
            return Err(LinkError::LinkAlreadyDisabled);
        }
        self.status = LinkStatus::Disabled;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Enable this link (requires valid credentials)
    pub fn enable(&mut self) -> Result<(), LinkError> {
        if self.status == LinkStatus::CredentialsExpired {
            return Err(LinkError::CredentialsExpired);
        }
        self.status = LinkStatus::Testing;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark credentials as expired
    #[allow(dead_code)]
    pub fn mark_credentials_expired(&mut self) {
        self.status = LinkStatus::CredentialsExpired;
        self.updated_at = Utc::now();
    }

    /// Update display name
    pub fn update_display_name(&mut self, name: String) {
        self.display_name = name;
        self.updated_at = Utc::now();
    }

    /// Compute SHA-256 hash of raw credentials for change detection
    pub fn compute_credentials_hash(credentials_json: &str) -> String {
        let digest = ring::digest::digest(&ring::digest::SHA256, credentials_json.as_bytes());
        hex::encode(digest.as_ref())
    }
}

// ─── Error Types ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("Link not found: {0}")]
    NotFound(Uuid),

    #[error("Credentials invalid: {0}")]
    CredentialsInvalid(String),

    #[error("Connection test failed: {0}")]
    ConnectionTestFailed(String),

    #[error("Link is already disabled")]
    LinkAlreadyDisabled,

    #[error("Link is already enabled")]
    LinkAlreadyEnabled,

    #[error("Max active links per connector exceeded")]
    MaxLinksPerConnector,

    #[error("Credentials expired")]
    CredentialsExpired,

    #[error("Connector not found: {0}")]
    ConnectorNotFound(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<LinkError> for platform_error::PlatformError {
    fn from(e: LinkError) -> Self {
        match e {
            LinkError::NotFound(id) => platform_error::PlatformError::NotFound { resource: "merchant_acquirer_link", id },
            LinkError::CredentialsInvalid(ref msg) | LinkError::ConnectionTestFailed(ref msg) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue { field: "credentials".into(), reason: msg.clone() }
                )
            }
            LinkError::LinkAlreadyDisabled | LinkError::LinkAlreadyEnabled | LinkError::MaxLinksPerConnector | LinkError::CredentialsExpired => {
                platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::IdempotencyKeyConflict
                )
            }
            LinkError::ConnectorNotFound(_c) => {
                platform_error::PlatformError::NotFound { resource: "connector", id: Uuid::nil() }
            }
            LinkError::EncryptionFailed(ref msg) => {
                platform_error::PlatformError::Internal(msg.clone())
            }
            LinkError::InvalidRequest(ref msg) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue { field: "request".into(), reason: msg.clone() }
                )
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_link() -> MerchantAcquirerLink {
        MerchantAcquirerLink::new(
            Uuid::now_v7(),
            "checkout_com".into(),
            "Production Gateway".into(),
            LinkEnvironment::Production,
            vec![1, 2, 3, 4],
            "abc123hash".into(),
        )
    }

    #[test]
    fn test_link_creation() {
        let link = create_test_link();
        assert_eq!(link.status, LinkStatus::Testing);
        assert_eq!(link.health_status, HealthStatus::Unknown);
        assert!(!link.status.is_routable());
    }

    #[test]
    fn test_connection_test_success() {
        let mut link = create_test_link();
        link.record_connection_test(true, 150);
        assert_eq!(link.status, LinkStatus::Active);
        assert_eq!(link.health_status, HealthStatus::Healthy);
        assert!(link.status.is_routable());
        assert!(link.last_tested_at.is_some());
    }

    #[test]
    fn test_connection_test_failure() {
        let mut link = create_test_link();
        link.record_connection_test(false, 0);
        assert_eq!(link.status, LinkStatus::Testing); // stays in testing
        assert_eq!(link.health_status, HealthStatus::Unreachable);
    }

    #[test]
    fn test_disable() {
        let mut link = create_test_link();
        link.record_connection_test(true, 100);
        assert!(link.disable().is_ok());
        assert_eq!(link.status, LinkStatus::Disabled);
    }

    #[test]
    fn test_double_disable_fails() {
        let mut link = create_test_link();
        link.disable().unwrap();
        assert!(link.disable().is_err());
    }

    #[test]
    fn test_enable_from_disabled() {
        let mut link = create_test_link();
        link.disable().unwrap();
        assert!(link.enable().is_ok());
        assert_eq!(link.status, LinkStatus::Testing);
    }

    #[test]
    fn test_rotate_credentials() {
        let mut link = create_test_link();
        link.record_connection_test(true, 100);
        link.rotate_credentials(vec![5, 6, 7, 8], "newhash".into());
        assert_eq!(link.status, LinkStatus::Testing); // reset to testing
        assert_eq!(link.credentials_hash, "newhash");
    }

    #[test]
    fn test_expired_credentials_cannot_enable() {
        let mut link = create_test_link();
        link.mark_credentials_expired();
        assert_eq!(link.status, LinkStatus::CredentialsExpired);
        assert!(link.enable().is_err());
    }

    #[test]
    fn test_credentials_hash_consistent() {
        let hash1 = MerchantAcquirerLink::compute_credentials_hash(r#"{"key":"value"}"#);
        let hash2 = MerchantAcquirerLink::compute_credentials_hash(r#"{"key":"value"}"#);
        assert_eq!(hash1, hash2);
    }
}
