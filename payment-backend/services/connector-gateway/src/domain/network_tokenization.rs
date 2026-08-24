//! Network Tokenization — reduced fees and improved authorization rates.
//!
//! Network tokens (provided by card schemes like Visa/Mastercard) offer:
//! - 15-25 bps lower interchange fees
//! - Higher authorization rates (3-5% improvement)
//! - Better card-on-file performance
//! - Reduced fraud risk

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Network Token Domain ────────────────────────────────────────────────────

/// A network token provisioned by a card scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkToken {
    pub token_id: Uuid,
    pub tenant_id: Uuid,
    pub card_fingerprint: String,
    pub network_token: String,
    pub token_expiry_month: u32,
    pub token_expiry_year: u32,
    pub cryptogram: Option<String>,
    pub network: CardNetwork,
    pub status: NetworkTokenStatus,
    pub provisioning_connector: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_count: u64,
}

/// Card network.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardNetwork {
    Visa,
    Mastercard,
    Amex,
    Discover,
    Jcb,
    UnionPay,
}

/// Network token status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkTokenStatus {
    Active,
    Suspended,
    Expired,
    Deactivated,
}

/// Token provision request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenProvisionRequest {
    pub tenant_id: Uuid,
    pub card_number: String,
    pub expiry_month: u32,
    pub expiry_year: u32,
    pub cardholder_name: Option<String>,
    pub billing_address: Option<Address>,
    pub preferred_connector: Option<String>,
}

/// Billing address for token provisioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// Result of network token provisioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenProvisionResult {
    pub token_id: Uuid,
    pub network_token: String,
    pub expiry_month: u32,
    pub expiry_year: u32,
    pub cryptogram: Option<String>,
    pub network: CardNetwork,
    pub estimated_fee_reduction_bps: i32,
}

/// Token updater result (when card details change).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenUpdate {
    pub token_id: Uuid,
    pub new_expiry_month: Option<u32>,
    pub new_expiry_year: Option<u32>,
    pub new_cryptogram: Option<String>,
    pub updated_at: DateTime<Utc>,
}

// ─── Network Token Service ───────────────────────────────────────────────────

/// Service for managing network tokens.
pub struct NetworkTokenService {
    /// Repository for token storage
    repository: Box<dyn NetworkTokenRepository>,
}

/// Repository trait for network tokens.
#[async_trait::async_trait]
pub trait NetworkTokenRepository: Send + Sync {
    async fn save(&self, token: &NetworkToken) -> Result<(), String>;
    async fn find_by_id(&self, token_id: Uuid) -> Result<Option<NetworkToken>, String>;
    async fn find_by_fingerprint(
        &self,
        tenant_id: Uuid,
        fingerprint: &str,
    ) -> Result<Option<NetworkToken>, String>;
    async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<NetworkToken>, String>;
    async fn update(&self, token: &NetworkToken) -> Result<(), String>;
}

impl NetworkTokenService {
    /// Create a new network token service.
    pub fn new(repository: Box<dyn NetworkTokenRepository>) -> Self {
        Self { repository }
    }

    /// Provision a new network token.
    pub async fn provision_token(
        &self,
        request: NetworkTokenProvisionRequest,
    ) -> Result<NetworkTokenProvisionResult, String> {
        // Check if token already exists for this card fingerprint
        let fingerprint = self.calculate_fingerprint(&request.card_number);

        if let Ok(Some(_existing)) = self
            .repository
            .find_by_fingerprint(request.tenant_id, &fingerprint)
            .await
        {
            return Err("Network token already exists for this card".into());
        }

        // In production, this would call the card scheme's token provisioning API
        // For now, generate a mock token
        let network_token = format!(
            "ntok_{}_{}",
            request.card_number[12..16].to_string(),
            Uuid::now_v7()
        );

        let token = NetworkToken {
            token_id: Uuid::now_v7(),
            tenant_id: request.tenant_id,
            card_fingerprint: fingerprint,
            network_token,
            token_expiry_month: request.expiry_month,
            token_expiry_year: request.expiry_year,
            cryptogram: Some(format!("crypt_{}", Uuid::now_v7())),
            network: self.detect_network(&request.card_number),
            status: NetworkTokenStatus::Active,
            provisioning_connector: request
                .preferred_connector
                .unwrap_or_else(|| "adyen".into()),
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
            usage_count: 0,
        };

        self.repository.save(&token).await?;

        Ok(NetworkTokenProvisionResult {
            token_id: token.token_id,
            network_token: token.network_token,
            expiry_month: token.token_expiry_month,
            expiry_year: token.token_expiry_year,
            cryptogram: token.cryptogram,
            network: token.network,
            estimated_fee_reduction_bps: 20, // Typical savings
        })
    }

    /// Get network token for a card.
    pub async fn get_token(
        &self,
        tenant_id: Uuid,
        card_fingerprint: &str,
    ) -> Result<Option<NetworkToken>, String> {
        self.repository
            .find_by_fingerprint(tenant_id, card_fingerprint)
            .await
    }

    /// List all network tokens for a tenant.
    pub async fn list_tokens(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<NetworkToken>, String> {
        self.repository.list_by_tenant(tenant_id).await
    }

    /// Update token (called by account updater).
    pub async fn update_token(
        &self,
        token_id: Uuid,
        update: NetworkTokenUpdate,
    ) -> Result<(), String> {
        let mut token = self
            .repository
            .find_by_id(token_id)
            .await?
            .ok_or("Token not found")?;

        if let Some(month) = update.new_expiry_month {
            token.token_expiry_month = month;
        }
        if let Some(year) = update.new_expiry_year {
            token.token_expiry_year = year;
        }
        if let Some(cryptogram) = update.new_cryptogram {
            token.cryptogram = Some(cryptogram);
        }

        self.repository.update(&token).await
    }

    /// Calculate card fingerprint (hash of card number).
    fn calculate_fingerprint(&self, card_number: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(card_number.as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..16]) // Use first 16 bytes
    }

    /// Detect card network from number.
    fn detect_network(&self, card_number: &str) -> CardNetwork {
        let first_digits: u32 = card_number[..6].parse().unwrap_or(0);

        match first_digits {
            400000..=499999 => CardNetwork::Visa,
            510000..=559999 => CardNetwork::Mastercard,
            340000..=379999 => CardNetwork::Amex,
            601100..=601199 => CardNetwork::Discover,
            352800..=358999 => CardNetwork::Jcb,
            620000..=629999 => CardNetwork::UnionPay,
            _ => CardNetwork::Visa, // Default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_network() {
        let service = NetworkTokenService::new(Box::new(MockTokenRepo));

        assert_eq!(service.detect_network("4111111111111111"), CardNetwork::Visa);
        assert_eq!(service.detect_network("5555555555554444"), CardNetwork::Mastercard);
        assert_eq!(service.detect_network("378282246310005"), CardNetwork::Amex);
    }

    #[test]
    fn test_calculate_fingerprint() {
        let service = NetworkTokenService::new(Box::new(MockTokenRepo));
        let fp1 = service.calculate_fingerprint("4111111111111111");
        let fp2 = service.calculate_fingerprint("4111111111111111");
        let fp3 = service.calculate_fingerprint("5555555555554444");

        assert_eq!(fp1, fp2); // Same card = same fingerprint
        assert_ne!(fp1, fp3); // Different card = different fingerprint
    }

    // Mock repository for tests
    struct MockTokenRepo;

    #[async_trait::async_trait]
    impl NetworkTokenRepository for MockTokenRepo {
        async fn save(&self, _token: &NetworkToken) -> Result<(), String> { Ok(()) }
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<NetworkToken>, String> { Ok(None) }
        async fn find_by_fingerprint(&self, _tenant_id: Uuid, _fp: &str) -> Result<Option<NetworkToken>, String> { Ok(None) }
        async fn list_by_tenant(&self, _tenant_id: Uuid) -> Result<Vec<NetworkToken>, String> { Ok(vec![]) }
        async fn update(&self, _token: &NetworkToken) -> Result<(), String> { Ok(()) }
    }
}
