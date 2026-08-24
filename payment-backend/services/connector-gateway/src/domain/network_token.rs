//! Network Token management for payment optimization.
//!
//! Network tokens (Visa/Mastercard tokens) provide:
//! - Higher authorization rates (2-5% improvement)
//! - Reduced fraud risk
//! - Automatic card updates via Account Updater
//! - Better cross-border transaction success

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Network token status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkTokenStatus {
    /// Token is active and can be used for transactions
    Active,
    /// Token is being provisioned
    Provisioning,
    /// Token has expired
    Expired,
    /// Token was revoked by the issuer
    Revoked,
    /// Token provisioning failed
    Failed,
}

impl std::fmt::Display for NetworkTokenStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Expired => write!(f, "expired"),
            Self::Revoked => write!(f, "revoked"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Network token network type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkType {
    Visa,
    Mastercard,
    Amex,
    Discover,
}

impl std::fmt::Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Visa => write!(f, "visa"),
            Self::Mastercard => write!(f, "mastercard"),
            Self::Amex => write!(f, "amex"),
            Self::Discover => write!(f, "discover"),
        }
    }
}

/// Network token aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkToken {
    pub token_id: Uuid,
    pub operator_id: Uuid,
    pub payment_method_token_id: Uuid,
    pub network: NetworkType,
    pub token_pan: String, // Masked PAN (e.g., "411111******1111")
    pub token_expiry_month: u32,
    pub token_expiry_year: u32,
    pub status: NetworkTokenStatus,
    pub provisioning_reference: Option<String>,
    pub network_token_reference: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl NetworkToken {
    /// Create a new network token request.
    pub fn new(
        token_id: Uuid,
        operator_id: Uuid,
        payment_method_token_id: Uuid,
        network: NetworkType,
        token_pan: String,
        expiry_month: u32,
        expiry_year: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            token_id,
            operator_id,
            payment_method_token_id,
            network,
            token_pan,
            token_expiry_month: expiry_month,
            token_expiry_year: expiry_year,
            status: NetworkTokenStatus::Provisioning,
            provisioning_reference: None,
            network_token_reference: None,
            is_default: false,
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    /// Check if the token is usable for transactions.
    pub fn is_usable(&self) -> bool {
        self.status == NetworkTokenStatus::Active
    }

    /// Mark token as provisioned successfully.
    pub fn mark_provisioned(&mut self, reference: String) {
        self.status = NetworkTokenStatus::Active;
        self.network_token_reference = Some(reference);
        self.updated_at = Utc::now();
    }

    /// Mark token provisioning as failed.
    pub fn mark_failed(&mut self) {
        self.status = NetworkTokenStatus::Failed;
        self.updated_at = Utc::now();
    }

    /// Revoke the token.
    pub fn revoke(&mut self) {
        self.status = NetworkTokenStatus::Revoked;
        self.updated_at = Utc::now();
    }

    /// Check if token is expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
}

/// Request to provision a network token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionNetworkTokenRequest {
    pub operator_id: Uuid,
    pub payment_method_token_id: Uuid,
    pub pan: String,
    pub expiry_month: u32,
    pub expiry_year: u32,
    pub cardholder_name: Option<String>,
}

/// Response from network token provisioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionNetworkTokenResponse {
    pub token_id: Uuid,
    pub status: NetworkTokenStatus,
    pub token_pan: Option<String>,
    pub provisioning_reference: Option<String>,
}

/// Request to check network token status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenStatusRequest {
    pub token_id: Uuid,
    pub provisioning_reference: Option<String>,
}

/// Request for Account Updater.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdaterRequest {
    pub token_id: Uuid,
    pub network_token_reference: String,
}

/// Account Updater response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdaterResponse {
    pub updated: bool,
    pub new_pan_last_four: Option<String>,
    pub new_expiry_month: Option<u32>,
    pub new_expiry_year: Option<u32>,
    pub reason: Option<String>,
}

/// Network token repository trait.
#[async_trait::async_trait]
pub trait NetworkTokenRepository: Send + Sync {
    /// Save a network token.
    async fn save(&self, token: &NetworkToken) -> Result<(), super::error::ConnectorError>;
    
    /// Load a network token by ID.
    async fn load(&self, token_id: Uuid) -> Result<Option<NetworkToken>, super::error::ConnectorError>;
    
    /// Load network tokens for a payment method token.
    async fn load_for_payment_method(
        &self,
        payment_method_token_id: Uuid,
    ) -> Result<Vec<NetworkToken>, super::error::ConnectorError>;
    
    /// Load the default network token for a payment method.
    async fn load_default(
        &self,
        payment_method_token_id: Uuid,
    ) -> Result<Option<NetworkToken>, super::error::ConnectorError>;
    
    /// Delete a network token.
    async fn delete(&self, token_id: Uuid) -> Result<(), super::error::ConnectorError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_token_creation() {
        let token = NetworkToken::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            NetworkType::Visa,
            "4111111111111111".to_string(),
            12,
            2025,
        );

        assert_eq!(token.status, NetworkTokenStatus::Provisioning);
        assert!(!token.is_usable());
    }

    #[test]
    fn test_network_token_provisioning() {
        let mut token = NetworkToken::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            NetworkType::Visa,
            "4111111111111111".to_string(),
            12,
            2025,
        );

        token.mark_provisioned("nt_ref_12345".to_string());
        assert_eq!(token.status, NetworkTokenStatus::Active);
        assert!(token.is_usable());
        assert_eq!(token.network_token_reference, Some("nt_ref_12345".to_string()));
    }

    #[test]
    fn test_network_token_revocation() {
        let mut token = NetworkToken::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            NetworkType::Mastercard,
            "5555555555554444".to_string(),
            12,
            2025,
        );

        token.mark_provisioned("nt_ref_67890".to_string());
        assert!(token.is_usable());

        token.revoke();
        assert_eq!(token.status, NetworkTokenStatus::Revoked);
        assert!(!token.is_usable());
    }

    #[test]
    fn test_network_token_expiry() {
        let mut token = NetworkToken::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            NetworkType::Visa,
            "4111111111111111".to_string(),
            12,
            2020, // Expired year
        );

        // Set expires_at to past
        token.expires_at = Some(chrono::DateTime::parse_from_rfc3339("2020-12-31T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc));
        
        assert!(token.is_expired());
    }
}
