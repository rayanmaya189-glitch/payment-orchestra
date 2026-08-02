//! Token Vault — portable payment method tokens across PSPs.
//!
//! Provides:
//! - PSP-agnostic token storage
//! - Token portability across connectors
//! - Automatic token refresh before expiry
//! - Token usage analytics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Token Domain Model ──────────────────────────────────────────────────────

/// A stored payment method token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodToken {
    pub token_id: Uuid,
    pub tenant_id: Uuid,
    pub token_type: TokenType,
    pub card_last_four: Option<String>,
    pub card_brand: Option<String>,
    pub expiry_month: Option<u32>,
    pub expiry_year: Option<u32>,
    pub issuer_country: Option<String>,
    pub fingerprint: Option<String>,
    /// Original PSP token references (one per PSP)
    pub psp_tokens: Vec<PspTokenReference>,
    pub status: TokenStatus,
    pub usage_count: u64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

/// Token type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenType {
    Card,
    BankAccount,
    Wallet,
    Upi,
}

/// A PSP-specific token reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PspTokenReference {
    pub connector_id: String,
    pub psp_token: String,
    pub psp_customer_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Token status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenStatus {
    Active,
    Expired,
    Revoked,
    Failed,
}

// ─── Vault Operations ────────────────────────────────────────────────────────

/// Result of tokenization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizationResult {
    pub token_id: Uuid,
    pub card_last_four: String,
    pub card_brand: String,
    pub expiry_month: u32,
    pub expiry_year: u32,
    pub fingerprint: String,
}

/// Vault interface for token operations.
#[async_trait::async_trait]
pub trait TokenVault: Send + Sync {
    /// Tokenize a payment method and store it.
    async fn tokenize(
        &self,
        tenant_id: Uuid,
        payment_data: &TokenizationRequest,
    ) -> Result<TokenizationResult, VaultError>;

    /// Retrieve a stored token.
    async fn get_token(
        &self,
        token_id: Uuid,
    ) -> Result<Option<PaymentMethodToken>, VaultError>;

    /// Get token by fingerprint (dedup).
    async fn get_by_fingerprint(
        &self,
        tenant_id: Uuid,
        fingerprint: &str,
    ) -> Result<Option<PaymentMethodToken>, VaultError>;

    /// List tokens for a tenant.
    async fn list_tokens(
        &self,
        tenant_id: Uuid,
        filters: TokenListFilters,
    ) -> Result<Vec<PaymentMethodToken>, VaultError>;

    /// Revoke a token.
    async fn revoke_token(
        &self,
        token_id: Uuid,
        reason: Option<String>,
    ) -> Result<(), VaultError>;

    /// Record token usage.
    async fn record_usage(
        &self,
        token_id: Uuid,
        connector_id: &str,
    ) -> Result<(), VaultError>;

    /// Get PSP token for a specific connector.
    async fn get_psp_token(
        &self,
        token_id: Uuid,
        connector_id: &str,
    ) -> Result<Option<PspTokenReference>, VaultError>;

    /// Add a PSP token reference (e.g., when first used with a new PSP).
    async fn add_psp_reference(
        &self,
        token_id: Uuid,
        reference: PspTokenReference,
    ) -> Result<(), VaultError>;
}

// ─── Request/Response Types ──────────────────────────────────────────────────

/// Request to tokenize a payment method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizationRequest {
    pub card_number: String,
    pub expiry_month: u32,
    pub expiry_year: u32,
    pub cvv: Option<String>,
    pub cardholder_name: Option<String>,
    pub billing_address: Option<Address>,
    pub metadata: Option<serde_json::Value>,
}

/// Billing address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// Filters for listing tokens.
#[derive(Debug, Clone, Default)]
pub struct TokenListFilters {
    pub status: Option<TokenStatus>,
    pub card_brand: Option<String>,
    pub expiring_before: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Vault errors.
#[derive(Debug, Clone)]
pub enum VaultError {
    TokenizationFailed(String),
    TokenNotFound(Uuid),
    TokenExpired(Uuid),
    TokenRevoked(Uuid),
    DuplicateFingerprint(String),
    StorageError(String),
    EncryptionError(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::TokenizationFailed(e) => write!(f, "Tokenization failed: {}", e),
            VaultError::TokenNotFound(id) => write!(f, "Token not found: {}", id),
            VaultError::TokenExpired(id) => write!(f, "Token expired: {}", id),
            VaultError::TokenRevoked(id) => write!(f, "Token revoked: {}", id),
            VaultError::DuplicateFingerprint(fp) => write!(f, "Duplicate fingerprint: {}", fp),
            VaultError::StorageError(e) => write!(f, "Storage error: {}", e),
            VaultError::EncryptionError(e) => write!(f, "Encryption error: {}", e),
        }
    }
}

impl std::error::Error for VaultError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_type_serialization() {
        let token_type = TokenType::Card;
        let json = serde_json::to_string(&token_type).unwrap();
        assert_eq!(json, "\"Card\"");
    }

    #[test]
    fn test_token_status_display() {
        assert_eq!(TokenStatus::Active.to_string(), "Active");
    }

    #[test]
    fn test_vault_error_display() {
        let err = VaultError::TokenNotFound(Uuid::nil());
        assert!(err.to_string().contains("not found"));
    }
}
