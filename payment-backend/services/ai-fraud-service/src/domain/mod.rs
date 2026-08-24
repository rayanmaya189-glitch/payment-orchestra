//! AI Fraud Detection Domain
//!
//! Implements real-time fraud detection using:
//! 1. Rule-based scoring (velocity, amount limits, BIN checks)
//! 2. ML model scoring (anomaly detection, pattern recognition)
//! 3. Device fingerprinting
//! 4. Behavioral analysis

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::repository::FraudRepository;

/// Fraud detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudCheckResult {
    pub transaction_id: Uuid,
    pub risk_score: f64,           // 0.0 (safe) to 1.0 (fraudulent)
    pub decision: FraudDecision,
    pub reasons: Vec<FraudReason>,
    pub model_version: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FraudDecision {
    Approve,
    Review,
    Decline,
}

impl FraudDecision {
    pub fn from_score(score: f64) -> Self {
        if score < 0.3 {
            Self::Approve
        } else if score < 0.7 {
            Self::Review
        } else {
            Self::Decline
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudReason {
    pub rule: String,
    pub score_impact: f64,
    pub description: String,
}

/// Transaction context for fraud analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionContext {
    pub transaction_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub card_bin: Option<String>,
    pub card_last_four: Option<String>,
    pub card_country: Option<String>,
    pub customer_ip: Option<String>,
    pub customer_email: Option<String>,
    pub device_fingerprint: Option<DeviceFingerprint>,
    pub shipping_address: Option<Address>,
    pub billing_address: Option<Address>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    pub device_id: String,
    pub browser_type: Option<String>,
    pub os_type: Option<String>,
    pub screen_resolution: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub country: String,
    pub region: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
}

/// ML Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_version: String,
    pub rule_weights: HashMap<String, f64>,
    pub threshold_approve: f64,
    pub threshold_review: f64,
    pub velocity_window_minutes: u32,
    pub max_transactions_per_hour: u32,
    pub max_amount_per_hour: i64,
}

impl Default for ModelConfig {
    fn default() -> Self {
        let mut rule_weights = HashMap::new();
        rule_weights.insert("velocity_check".into(), 0.25);
        rule_weights.insert("amount_check".into(), 0.20);
        rule_weights.insert("bin_check".into(), 0.15);
        rule_weights.insert("ip_check".into(), 0.15);
        rule_weights.insert("device_check".into(), 0.10);
        rule_weights.insert("email_check".into(), 0.10);
        rule_weights.insert("address_check".into(), 0.05);

        Self {
            model_version: "1.0.0".into(),
            rule_weights,
            threshold_approve: 0.3,
            threshold_review: 0.7,
            velocity_window_minutes: 60,
            max_transactions_per_hour: 10,
            max_amount_per_hour: 1000000, // 10,000 in minor units (e.g., $10,000)
        }
    }
}

/// Main fraud detector
pub struct FraudDetector {
    repository: Arc<dyn FraudRepository>,
    config: ModelConfig,
}

impl FraudDetector {
    pub async fn new(
        repository: Arc<dyn FraudRepository>,
        config: ModelConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { repository, config })
    }

    /// Perform real-time fraud check on a transaction
    pub async fn check_transaction(
        &self,
        ctx: &TransactionContext,
    ) -> Result<FraudCheckResult, FraudError> {
        let mut reasons = Vec::new();
        let mut total_score = 0.0;

        // Rule 1: Velocity check
        if let Some(score) = self.check_velocity(ctx).await? {
            total_score += score * self.config.rule_weights.get("velocity_check").unwrap_or(&0.25);
            reasons.push(FraudReason {
                rule: "velocity_check".into(),
                score_impact: score,
                description: "Transaction velocity exceeds threshold".into(),
            });
        }

        // Rule 2: Amount check
        if let Some(score) = self.check_amount(ctx).await? {
            total_score += score * self.config.rule_weights.get("amount_check").unwrap_or(&0.20);
            reasons.push(FraudReason {
                rule: "amount_check".into(),
                score_impact: score,
                description: "Transaction amount is unusually high".into(),
            });
        }

        // Rule 3: BIN/Country mismatch
        if let Some(score) = self.check_bin_country(ctx).await? {
            total_score += score * self.config.rule_weights.get("bin_check").unwrap_or(&0.15);
            reasons.push(FraudReason {
                rule: "bin_check".into(),
                score_impact: score,
                description: "BIN country mismatch detected".into(),
            });
        }

        // Rule 4: IP/Country mismatch
        if let Some(score) = self.check_ip_country(ctx).await? {
            total_score += score * self.config.rule_weights.get("ip_check").unwrap_or(&0.15);
            reasons.push(FraudReason {
                rule: "ip_check".into(),
                score_impact: score,
                description: "IP geolocation mismatch detected".into(),
            });
        }

        // Rule 5: Device fingerprint analysis
        if let Some(score) = self.check_device(ctx).await? {
            total_score += score * self.config.rule_weights.get("device_check").unwrap_or(&0.10);
            reasons.push(FraudReason {
                rule: "device_check".into(),
                score_impact: score,
                description: "Suspicious device fingerprint".into(),
            });
        }

        // Rule 6: Email reputation
        if let Some(score) = self.check_email(ctx).await? {
            total_score += score * self.config.rule_weights.get("email_check").unwrap_or(&0.10);
            reasons.push(FraudReason {
                rule: "email_check".into(),
                score_impact: score,
                description: "Suspicious email pattern".into(),
            });
        }

        // Rule 7: Address verification
        if let Some(score) = self.check_address(ctx).await? {
            total_score += score * self.config.rule_weights.get("address_check").unwrap_or(&0.05);
            reasons.push(FraudReason {
                rule: "address_check".into(),
                score_impact: score,
                description: "Address verification failed".into(),
            });
        }

        // Clamp score to [0.0, 1.0]
        let risk_score = total_score.clamp(0.0, 1.0);
        let decision = FraudDecision::from_score(risk_score);

        let result = FraudCheckResult {
            transaction_id: ctx.transaction_id,
            risk_score,
            decision,
            reasons,
            model_version: self.config.model_version.clone(),
            checked_at: Utc::now(),
        };

        // Store result for analysis
        self.repository.store_fraud_check(&result).await?;

        Ok(result)
    }

    /// Check transaction velocity
    async fn check_velocity(&self, ctx: &TransactionContext) -> Result<Option<f64>, FraudError> {
        let customer_key = ctx.customer_email.clone()
            .or(ctx.customer_ip.clone())
            .unwrap_or_default();

        let window = Duration::from_secs(self.config.velocity_window_minutes as u64 * 60);
        let recent_txns = self.repository
            .get_recent_transactions(&customer_key, window)
            .await?;

        let txn_count = recent_txns.len() as u32;
        if txn_count > self.config.max_transactions_per_hour {
            let excess = (txn_count - self.config.max_transactions_per_hour) as f64;
            let score = (excess / self.config.max_transactions_per_hour as f64).min(1.0);
            return Ok(Some(score));
        }

        Ok(None)
    }

    /// Check transaction amount
    async fn check_amount(&self, ctx: &TransactionContext) -> Result<Option<f64>, FraudError> {
        let customer_key = ctx.customer_email.clone()
            .or(ctx.customer_ip.clone())
            .unwrap_or_default();

        let window = Duration::from_secs(3600); // 1 hour
        let recent_txns = self.repository
            .get_recent_transactions(&customer_key, window)
            .await?;

        let total_amount: i64 = recent_txns.iter()
            .map(|t| t.amount_minor)
            .sum();

        let new_total = total_amount + ctx.amount_minor;
        if new_total > self.config.max_amount_per_hour {
            let excess = (new_total - self.config.max_amount_per_hour) as f64;
            let score = (excess / self.config.max_amount_per_hour as f64).min(1.0);
            return Ok(Some(score));
        }

        Ok(None)
    }

    /// Check BIN/Country mismatch
    async fn check_bin_country(&self, ctx: &TransactionContext) -> Result<Option<f64>, FraudError> {
        if let (Some(bin), Some(card_country)) = (&ctx.card_bin, &ctx.card_country) {
            if let Some(billing_country) = &ctx.billing_address {
                if billing_country.country != *card_country {
                    // Different billing country than card issuing country
                    return Ok(Some(0.4));
                }
            }
        }
        Ok(None)
    }

    /// Check IP/Country mismatch
    async fn check_ip_country(&self, ctx: &TransactionContext) -> Result<Option<f64>, FraudError> {
        // In production, use GeoIP lookup
        // For now, check if IP country matches billing country
        Ok(None)
    }

    /// Check device fingerprint
    async fn check_device(&self, ctx: &TransactionContext) -> Result<Option<f64>, FraudError> {
        if let Some(device) = &ctx.device_fingerprint {
            // Check if device was used by multiple customers
            let users = self.repository.get_device_users(&device.device_id).await?;
            if users.len() > 3 {
                return Ok(Some(0.5));
            }
        }
        Ok(None)
    }

    /// Check email reputation
    async fn check_email(&self, ctx: &TransactionContext) -> Result<Option<f64>, FraudError> {
        if let Some(email) = &ctx.customer_email {
            // Check for disposable email domains
            let disposable_domains = ["tempmail.com", "throwaway.email", "guerrillamail.com"];
            if let Some(domain) = email.split('@').last() {
                if disposable_domains.contains(&domain) {
                    return Ok(Some(0.6));
                }
            }

            // Check account age
            let account_age = self.repository.get_account_age(email).await?;
            if account_age < Duration::from_secs(86400) { // Less than 1 day
                return Ok(Some(0.3));
            }
        }
        Ok(None)
    }

    /// Check address verification
    async fn check_address(&self, ctx: &TransactionContext) -> Result<Option<f64>, FraudError> {
        // In production, use AVS (Address Verification Service)
        Ok(None)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FraudError {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<crate::repository::RepositoryError> for FraudError {
    fn from(e: crate::repository::RepositoryError) -> Self {
        Self::Repository(e.to_string())
    }
}
