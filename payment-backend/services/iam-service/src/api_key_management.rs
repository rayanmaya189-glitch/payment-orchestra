//! API Key Management — CRUD operations and key rotation.
//!
//! Provides:
//! - Create new API keys with scopes
//! - List API keys for a principal
//! - Revoke API keys
//! - Rotate API keys (create new, revoke old)
//! - Key prefix for fast lookup

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── API Key Domain Model ────────────────────────────────────────────────────

/// API key status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

impl std::fmt::Display for ApiKeyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKeyStatus::Active => write!(f, "active"),
            ApiKeyStatus::Revoked => write!(f, "revoked"),
            ApiKeyStatus::Expired => write!(f, "expired"),
        }
    }
}

/// An API key issued to a principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub name: String,
    pub key_prefix: String, // First 12 chars: "pk_live_abc"
    pub key_hash: Vec<u8>,  // Argon2 hash of the full key
    pub scopes: Vec<String>,
    pub status: ApiKeyStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub rotated_from: Option<Uuid>, // Previous key ID if rotated
}

/// API key with the raw key visible (only returned on creation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCreated {
    pub api_key: ApiKey,
    pub raw_key: String, // Full key, only shown once
}

/// API key summary (without hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeySummary {
    pub api_key_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub status: ApiKeyStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

// ─── Commands ────────────────────────────────────────────────────────────────

/// Command to create a new API key.
#[derive(Debug, Clone)]
pub struct CreateApiKey {
    pub principal_id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

/// Command to rotate an API key.
#[derive(Debug, Clone)]
pub struct RotateApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub grace_period_hours: u32, // Hours both keys remain valid
}

/// Command to revoke an API key.
#[derive(Debug, Clone)]
pub struct RevokeApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub reason: Option<String>,
}

// ─── Query Types ─────────────────────────────────────────────────────────────

/// Query to list API keys for a principal.
#[derive(Debug, Clone)]
pub struct ListApiKeys {
    pub principal_id: Uuid,
    pub include_revoked: bool,
}

// ─── Results ─────────────────────────────────────────────────────────────────

/// Result of API key creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyResult {
    pub api_key_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Result of API key rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRotationResult {
    pub new_key: ApiKeyResult,
    pub raw_key: String, // Full new key, only shown once
    pub old_key_id: Uuid,
    pub grace_period_ends_at: DateTime<Utc>,
}

// ─── Key Generation ──────────────────────────────────────────────────────────

/// Generate a new API key with prefix.
///
/// Returns (raw_key, prefix, hash).
pub fn generate_api_key(prefix: &str) -> (String, String, Vec<u8>) {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let random_hex = hex::encode(&random_bytes);

    let raw_key = format!("{}_{}", prefix, random_hex);
    let key_prefix = raw_key[..prefix.len() + 9].to_string(); // "pk_live_" + first 8 chars

    // Hash the key with Argon2
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash = Argon2::default()
        .hash_password(raw_key.as_bytes(), &salt)
        .unwrap()
        .to_string()
        .into_bytes();

    (raw_key, key_prefix, hash)
}

/// Determine the key prefix based on environment.
pub fn key_prefix_for_env(environment: &str) -> &str {
    match environment {
        "production" => "pk_live",
        "sandbox" => "pk_test",
        _ => "pk_test",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let (raw, prefix, hash) = generate_api_key("pk_live");
        assert!(raw.starts_with("pk_live_"));
        assert!(prefix.starts_with("pk_live_"));
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_key_prefix_for_env() {
        assert_eq!(key_prefix_for_env("production"), "pk_live");
        assert_eq!(key_prefix_for_env("sandbox"), "pk_test");
        assert_eq!(key_prefix_for_env("unknown"), "pk_test");
    }

    #[test]
    fn test_api_key_status_display() {
        assert_eq!(ApiKeyStatus::Active.to_string(), "active");
        assert_eq!(ApiKeyStatus::Revoked.to_string(), "revoked");
        assert_eq!(ApiKeyStatus::Expired.to_string(), "expired");
    }
}
