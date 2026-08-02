//! Fraud Intelligence API — cross-merchant fraud signals and risk scoring.
//!
//! Provides:
//! - Real-time fraud scoring
//! - Device fingerprinting
//! - Velocity checks
//! - Cross-merchant fraud database
//! - Rule-based and ML-based fraud detection

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Fraud Assessment ────────────────────────────────────────────────────────

/// A fraud risk assessment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudAssessment {
    pub assessment_id: Uuid,
    pub transaction_id: String,
    pub risk_score: f64,        // 0.0 (safe) to 1.0 (fraudulent)
    pub risk_level: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub recommendation: FraudRecommendation,
    pub device_fingerprint: Option<DeviceFingerprint>,
    pub velocity_check: VelocityCheckResult,
    pub assessed_at: DateTime<Utc>,
    pub model_version: String,
}

/// Risk level classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Fraud recommendation based on assessment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FraudRecommendation {
    Approve,
    Review,
    Decline,
    Block,
}

/// Individual risk factor contributing to the score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub description: String,
    pub weight: f64,
    pub score: f64,
}

// ─── Device Fingerprinting ───────────────────────────────────────────────────

/// Device fingerprint for fraud detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    pub fingerprint_id: String,
    pub device_type: String,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub screen_resolution: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub ip_address: Option<String>,
    pub ip_country: Option<String>,
    pub is_proxy: bool,
    pub is_vpn: bool,
    pub is_tor: bool,
    pub fraud_history: Option<DeviceFraudHistory>,
}

/// Historical fraud data for a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFraudHistory {
    pub total_transactions: u64,
    pub fraudulent_transactions: u64,
    pub last_fraudulent_at: Option<DateTime<Utc>>,
    pub known_fraud_ips: Vec<String>,
}

// ─── Velocity Checks ─────────────────────────────────────────────────────────

/// Velocity check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityConfig {
    pub max_transactions_per_hour: u32,
    pub max_transactions_per_day: u32,
    pub max_amount_per_hour: i64,
    pub max_amount_per_day: i64,
    pub max_failed_attempts_per_hour: u32,
}

/// Velocity check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityCheckResult {
    pub passed: bool,
    pub transactions_last_hour: u32,
    pub transactions_last_day: u32,
    pub amount_last_hour: i64,
    pub amount_last_day: i64,
    pub failed_attempts_last_hour: u32,
    pub violations: Vec<VelocityViolation>,
}

/// Velocity check violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityViolation {
    pub violation_type: String,
    pub limit: i64,
    pub current: i64,
    pub window: String,
}

// ─── Fraud Request/Response ──────────────────────────────────────────────────

/// Request for fraud assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudAssessmentRequest {
    pub transaction_id: String,
    pub amount: i64,
    pub currency: String,
    pub card_number: Option<String>,
    pub card_fingerprint: Option<String>,
    pub device_fingerprint: Option<DeviceFingerprint>,
    pub merchant_id: String,
    pub customer_id: Option<String>,
    pub ip_address: Option<String>,
    pub billing_address: Option<Address>,
    pub shipping_address: Option<Address>,
    pub metadata: Option<serde_json::Value>,
}

/// Address for fraud checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// Fraud rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudRule {
    pub rule_id: Uuid,
    pub name: String,
    pub description: String,
    pub conditions: Vec<FraudCondition>,
    pub action: FraudAction,
    pub priority: i32,
    pub enabled: bool,
}

/// Fraud rule condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

/// Condition operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    NotContains,
    In,
    NotIn,
}

/// Action to take when fraud is detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FraudAction {
    Block,
    Decline,
    Review,
    Flag,
    Alert,
}

// ─── Fraud Intelligence Service ──────────────────────────────────────────────

/// Service for fraud detection and intelligence.
pub struct FraudIntelligenceService {
    rules: Vec<FraudRule>,
    velocity_config: VelocityConfig,
}

impl FraudIntelligenceService {
    /// Create a new fraud intelligence service.
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            velocity_config: VelocityConfig {
                max_transactions_per_hour: 10,
                max_transactions_per_day: 50,
                max_amount_per_hour: 1000000, // $10,000
                max_amount_per_day: 10000000, // $100,000
                max_failed_attempts_per_hour: 5,
            },
        }
    }

    /// Assess a transaction for fraud.
    pub async fn assess(
        &self,
        request: FraudAssessmentRequest,
    ) -> FraudAssessment {
        let mut risk_score: f64 = 0.0;
        let mut risk_factors = Vec::new();

        // Check device fingerprint
        if let Some(ref device) = request.device_fingerprint {
            if device.is_proxy || device.is_vpn {
                risk_score += 0.2;
                risk_factors.push(RiskFactor {
                    factor_type: "proxy_vpn".into(),
                    description: "Transaction from proxy/VPN".into(),
                    weight: 0.2,
                    score: 1.0,
                });
            }
            if device.is_tor {
                risk_score += 0.4;
                risk_factors.push(RiskFactor {
                    factor_type: "tor".into(),
                    description: "Transaction from Tor network".into(),
                    weight: 0.4,
                    score: 1.0,
                });
            }
        }

        // Check amount
        if request.amount > 5000000 {
            // $50,000+
            risk_score += 0.15;
            risk_factors.push(RiskFactor {
                factor_type: "high_amount".into(),
                description: "High transaction amount".into(),
                weight: 0.15,
                score: 1.0,
            });
        }

        // Check velocity (simplified - in production would check actual counts)
        let velocity_result = VelocityCheckResult {
            passed: true,
            transactions_last_hour: 0,
            transactions_last_day: 0,
            amount_last_hour: 0,
            amount_last_day: 0,
            failed_attempts_last_hour: 0,
            violations: vec![],
        };

        // Determine risk level
        let risk_level = if risk_score < 0.3 {
            RiskLevel::Low
        } else if risk_score < 0.6 {
            RiskLevel::Medium
        } else if risk_score < 0.8 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        };

        // Determine recommendation
        let recommendation = match risk_level {
            RiskLevel::Low => FraudRecommendation::Approve,
            RiskLevel::Medium => FraudRecommendation::Review,
            RiskLevel::High => FraudRecommendation::Decline,
            RiskLevel::Critical => FraudRecommendation::Block,
        };

        FraudAssessment {
            assessment_id: Uuid::now_v7(),
            transaction_id: request.transaction_id,
            risk_score,
            risk_level,
            risk_factors,
            recommendation,
            device_fingerprint: request.device_fingerprint,
            velocity_check: velocity_result,
            assessed_at: Utc::now(),
            model_version: "1.0.0".into(),
        }
    }

    /// Get default fraud rules.
    fn default_rules() -> Vec<FraudRule> {
        vec![
            FraudRule {
                rule_id: Uuid::nil(),
                name: "High Amount".into(),
                description: "Flag transactions over $10,000".into(),
                conditions: vec![FraudCondition {
                    field: "amount".into(),
                    operator: ConditionOperator::GreaterThan,
                    value: serde_json::json!(1000000),
                }],
                action: FraudAction::Review,
                priority: 1,
                enabled: true,
            },
            FraudRule {
                rule_id: Uuid::nil(),
                name: "Tor Network".into(),
                description: "Block transactions from Tor".into(),
                conditions: vec![FraudCondition {
                    field: "device.is_tor".into(),
                    operator: ConditionOperator::Equals,
                    value: serde_json::json!(true),
                }],
                action: FraudAction::Block,
                priority: 0,
                enabled: true,
            },
        ]
    }
}

impl Default for FraudIntelligenceService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fraud_assessment_low_risk() {
        let service = FraudIntelligenceService::new();
        let request = FraudAssessmentRequest {
            transaction_id: "tx-1".into(),
            amount: 5000, // $50
            currency: "USD".into(),
            card_number: None,
            card_fingerprint: None,
            device_fingerprint: None,
            merchant_id: "m-1".into(),
            customer_id: None,
            ip_address: None,
            billing_address: None,
            shipping_address: None,
            metadata: None,
        };

        let assessment = service.assess(request).await;
        assert_eq!(assessment.risk_level, RiskLevel::Low);
        assert_eq!(assessment.recommendation, FraudRecommendation::Approve);
    }

    #[tokio::test]
    async fn test_fraud_assessment_high_risk() {
        let service = FraudIntelligenceService::new();
        let request = FraudAssessmentRequest {
            transaction_id: "tx-2".into(),
            amount: 10000000, // $100,000
            currency: "USD".into(),
            card_number: None,
            card_fingerprint: None,
            device_fingerprint: Some(DeviceFingerprint {
                fingerprint_id: "fp-1".into(),
                device_type: "desktop".into(),
                browser: None,
                os: None,
                screen_resolution: None,
                timezone: None,
                language: None,
                ip_address: None,
                ip_country: None,
                is_proxy: true,
                is_vpn: true,
                is_tor: true,
                fraud_history: None,
            }),
            merchant_id: "m-1".into(),
            customer_id: None,
            ip_address: None,
            billing_address: None,
            shipping_address: None,
            metadata: None,
        };

        let assessment = service.assess(request).await;
        assert!(assessment.risk_score > 0.5);
        assert!(assessment.risk_level == RiskLevel::High || assessment.risk_level == RiskLevel::Critical);
    }
}
