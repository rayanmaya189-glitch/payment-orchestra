//! Webhook aggregate — registered endpoint for event-driven notifications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A registered webhook endpoint for event-driven notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub webhook_id: Uuid,
    pub operator_id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub secret_hash: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Webhook {
    /// Create a new webhook registration. Returns (Self, raw_secret).
    /// The raw secret should be returned to the caller exactly once for HMAC signing.
    pub fn new(
        operator_id: Uuid,
        url: String,
        events: Vec<String>,
    ) -> (Self, String) {
        // Generate a random secret for HMAC signing
        let raw_secret = Uuid::now_v7().to_string() + &Uuid::now_v7().to_string();
        let secret_hash = Self::hash_secret(&raw_secret);

        let webhook = Self {
            webhook_id: Uuid::now_v7(),
            operator_id,
            url,
            events,
            secret_hash,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        (webhook, raw_secret)
    }

    /// Hash a secret for storage (SHA-256).
    fn hash_secret(secret: &str) -> String {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(secret.as_bytes());
        hex::encode(hash)
    }

    /// Verify a provided secret against the stored hash.
    pub fn verify_secret(&self, secret: &str) -> bool {
        Self::hash_secret(secret) == self.secret_hash
    }

    /// Deactivate this webhook.
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }
}
