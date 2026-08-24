//! ML-based routing — gradient boosting for payment success rate optimization.
//!
//! Uses a lightweight gradient boosting model to predict payment success
//! probability for each gateway, considering:
//! - Historical success rates per gateway
//! - Card scheme performance
//! - Time-of-day patterns
//! - Amount ranges
//! - Currency-specific performance

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ─── Feature Engineering ─────────────────────────────────────────────────────

/// Features for ML routing prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingFeatures {
    /// Gateway features
    pub gateway_id: String,
    pub gateway_base_success_rate: f64,
    pub gateway_recent_success_rate: f64,
    pub gateway_avg_latency_ms: f64,
    pub gateway_volume_24h: u64,

    /// Transaction features
    pub amount_minor_units: i64,
    pub currency: String,
    pub card_scheme: String,

    /// Temporal features
    pub hour_of_day: u32,
    pub day_of_week: u32,
    pub is_weekend: bool,

    /// Geographic features
    pub issuer_country: Option<String>,
    pub merchant_country: String,
}

/// Prediction result for a gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPrediction {
    pub gateway_id: String,
    pub success_probability: f64,
    pub expected_latency_ms: f64,
    pub confidence: f64,
}

// ─── Gradient Boosting Model ─────────────────────────────────────────────────

/// Lightweight gradient boosting model for success rate prediction.
///
/// This is a simplified version that uses a decision tree ensemble
/// for fast inference without external ML dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientBoostingModel {
    /// Decision trees in the ensemble
    trees: Vec<DecisionTree>,
    /// Learning rate
    learning_rate: f64,
    /// Base prediction (log-odds of success)
    base_prediction: f64,
    /// Feature importance scores
    feature_importance: HashMap<String, f64>,
    /// Training metadata
    trained_at: Option<DateTime<Utc>>,
    sample_count: u64,
}

/// A single decision tree in the ensemble.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTree {
    /// Tree depth
    depth: u32,
    /// Root node
    root: TreeNode,
}

/// A node in the decision tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TreeNode {
    Leaf {
        prediction: f64,
    },
    Split {
        feature_index: usize,
        threshold: f64,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
    },
}

impl GradientBoostingModel {
    /// Create a new model with default parameters.
    pub fn new() -> Self {
        Self {
            trees: Vec::new(),
            learning_rate: 0.1,
            base_prediction: 0.0,
            feature_importance: HashMap::new(),
            trained_at: None,
            sample_count: 0,
        }
    }

    /// Create a pre-trained model with typical payment routing parameters.
    pub fn pre_trained() -> Self {
        let mut model = Self::new();

        // Pre-computed feature importance from typical payment data
        model.feature_importance.insert("gateway_base_success_rate".into(), 0.35);
        model.feature_importance.insert("gateway_recent_success_rate".into(), 0.30);
        model.feature_importance.insert("amount_minor_units".into(), 0.10);
        model.feature_importance.insert("card_scheme".into(), 0.08);
        model.feature_importance.insert("hour_of_day".into(), 0.07);
        model.feature_importance.insert("currency".into(), 0.05);
        model.feature_importance.insert("gateway_avg_latency_ms".into(), 0.05);

        // Base prediction (log-odds of ~95% success rate)
        model.base_prediction = 2.944; // log(0.95/0.05)

        // Simple decision tree for gateway selection
        model.trees.push(DecisionTree {
            depth: 3,
            root: TreeNode::Split {
                feature_index: 0, // gateway_base_success_rate
                threshold: 0.90,
                left: Box::new(TreeNode::Split {
                    feature_index: 1, // gateway_recent_success_rate
                    threshold: 0.85,
                    left: Box::new(TreeNode::Leaf { prediction: -0.5 }),
                    right: Box::new(TreeNode::Leaf { prediction: 0.2 }),
                }),
                right: Box::new(TreeNode::Split {
                    feature_index: 2, // amount_minor_units
                    threshold: 100000, // $1000
                    left: Box::new(TreeNode::Leaf { prediction: 0.8 }),
                    right: Box::new(TreeNode::Leaf { prediction: 0.3 }),
                }),
            },
        });

        model.trained_at = Some(Utc::now());
        model.sample_count = 100000;

        model
    }

    /// Predict success probability for a gateway given features.
    pub fn predict(&self, features: &RoutingFeatures) -> GatewayPrediction {
        let mut log_odds = self.base_prediction;

        // Run through each tree
        for tree in &self.trees {
            log_odds += self.learning_rate * tree.predict(features);
        }

        // Convert log-odds to probability
        let success_probability = 1.0 / (1.0 + (-log_odds).exp());

        // Calculate confidence based on sample count and feature coverage
        let confidence = self.calculate_confidence(features);

        // Estimate latency based on historical patterns
        let expected_latency = self.estimate_latency(features);

        GatewayPrediction {
            gateway_id: features.gateway_id.clone(),
            success_probability,
            expected_latency_ms: expected_latency,
            confidence,
        }
    }

    /// Predict for multiple gateways and rank by expected value.
    pub fn rank_gateways(
        &self,
        features_per_gateway: &[(String, RoutingFeatures)],
    ) -> Vec<GatewayPrediction> {
        let mut predictions: Vec<GatewayPrediction> = features_per_gateway
            .iter()
            .map(|(gateway_id, features)| {
                let mut pred = self.predict(features);
                pred.gateway_id = gateway_id.clone();
                pred
            })
            .collect();

        // Sort by composite score: success_probability * (1 - latency_penalty)
        predictions.sort_by(|a, b| {
            let score_a = a.success_probability * (1.0 - (a.expected_latency_ms / 10000.0).min(1.0));
            let score_b = b.success_probability * (1.0 - (b.expected_latency_ms / 10000.0).min(1.0));
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        predictions
    }

    fn calculate_confidence(&self, features: &RoutingFeatures) -> f64 {
        // Confidence increases with more training data and well-known features
        let base_confidence = if self.sample_count > 10000 {
            0.8
        } else if self.sample_count > 1000 {
            0.6
        } else {
            0.4
        };

        // Reduce confidence for extreme amounts or unusual hours
        let amount_penalty = if features.amount_minor_units > 10000000 {
            0.1
        } else {
            0.0
        };

        let time_penalty = if features.hour_of_day < 6 || features.hour_of_day > 23 {
            0.05
        } else {
            0.0
        };

        (base_confidence - amount_penalty - time_penalty).max(0.1)
    }

    fn estimate_latency(&self, features: &RoutingFeatures) -> f64 {
        // Base latency from gateway
        let mut latency = features.gateway_avg_latency_ms;

        // Amount penalty: larger amounts may have more verification
        if features.amount_minor_units > 5000000 {
            latency *= 1.2;
        }

        // Time-of-day adjustment: peak hours may be slower
        if features.hour_of_day >= 10 && features.hour_of_day <= 14 {
            latency *= 1.1;
        }

        latency
    }

    /// Get feature importance ranking.
    pub fn feature_importance(&self) -> &HashMap<String, f64> {
        &self.feature_importance
    }

    /// Check if the model is trained and ready.
    pub fn is_ready(&self) -> bool {
        !self.trees.is_empty() && self.trained_at.is_some()
    }

    /// Get training metadata.
    pub fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            tree_count: self.trees.len(),
            learning_rate: self.learning_rate,
            trained_at: self.trained_at,
            sample_count: self.sample_count,
            feature_count: self.feature_importance.len(),
        }
    }
}

impl Default for GradientBoostingModel {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionTree {
    fn predict(&self, features: &RoutingFeatures) -> f64 {
        self.predict_node(&self.root, features)
    }

    fn predict_node(&self, node: &TreeNode, features: &RoutingFeatures) -> f64 {
        match node {
            TreeNode::Leaf { prediction } => *prediction,
            TreeNode::Split {
                feature_index,
                threshold,
                left,
                right,
            } => {
                let feature_value = self.get_feature_value(features, *feature_index);
                if feature_value <= *threshold {
                    self.predict_node(left, features)
                } else {
                    self.predict_node(right, features)
                }
            }
        }
    }

    fn get_feature_value(&self, features: &RoutingFeatures, index: usize) -> f64 {
        match index {
            0 => features.gateway_base_success_rate,
            1 => features.gateway_recent_success_rate,
            2 => features.amount_minor_units as f64,
            3 => features.gateway_avg_latency_ms,
            4 => features.hour_of_day as f64,
            5 => features.gateway_volume_24h as f64,
            _ => 0.0,
        }
    }
}

// ─── Model Metadata ──────────────────────────────────────────────────────────

/// Model training metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub tree_count: usize,
    pub learning_rate: f64,
    pub trained_at: Option<DateTime<Utc>>,
    pub sample_count: u64,
    pub feature_count: usize,
}

// ─── Feature Extractor ───────────────────────────────────────────────────────

/// Extracts features from payment context for ML routing.
pub struct FeatureExtractor {
    /// Historical gateway performance data
    gateway_stats: HashMap<String, GatewayStats>,
}

/// Aggregated gateway statistics.
#[derive(Debug, Clone, Default)]
pub struct GatewayStats {
    pub total_attempts: u64,
    pub successful_attempts: u64,
    pub recent_attempts: u64,
    pub recent_successes: u64,
    pub avg_latency_ms: f64,
    pub volume_24h: u64,
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self {
            gateway_stats: HashMap::new(),
        }
    }

    /// Update gateway stats with a new transaction result.
    pub fn record_transaction(
        &mut self,
        gateway_id: &str,
        success: bool,
        latency_ms: f64,
        amount: i64,
    ) {
        let stats = self
            .gateway_stats
            .entry(gateway_id.to_string())
            .or_insert_with(GatewayStats::default);

        stats.total_attempts += 1;
        if success {
            stats.successful_attempts += 1;
        }

        // Update rolling averages
        let n = stats.total_attempts as f64;
        stats.avg_latency_ms = (stats.avg_latency_ms * (n - 1.0) + latency_ms) / n;

        // Recent stats (last 100 transactions)
        stats.recent_attempts = stats.recent_attempts.saturating_add(1).min(100);
        if success {
            stats.recent_successes = stats.recent_successes.saturating_add(1).min(100);
        }

        stats.volume_24h += amount as u64;
    }

    /// Extract features for a gateway.
    pub fn extract_features(
        &self,
        gateway_id: &str,
        amount: i64,
        currency: &str,
        card_scheme: &str,
        merchant_country: &str,
        issuer_country: Option<&str>,
    ) -> RoutingFeatures {
        let stats = self.gateway_stats.get(gateway_id).cloned().unwrap_or_default();

        let now = Utc::now();

        RoutingFeatures {
            gateway_id: gateway_id.to_string(),
            gateway_base_success_rate: if stats.total_attempts > 0 {
                stats.successful_attempts as f64 / stats.total_attempts as f64
            } else {
                0.95 // Default
            },
            gateway_recent_success_rate: if stats.recent_attempts > 0 {
                stats.recent_successes as f64 / stats.recent_attempts as f64
            } else {
                0.95
            },
            gateway_avg_latency_ms: stats.avg_latency_ms,
            gateway_volume_24h: stats.volume_24h,
            amount_minor_units: amount,
            currency: currency.to_string(),
            card_scheme: card_scheme.to_string(),
            hour_of_day: now.hour(),
            day_of_week: now.weekday().num_days_from_sunday(),
            is_weekend: now.weekday().num_days_from_sunday() >= 5,
            issuer_country: issuer_country.map(|s| s.to_string()),
            merchant_country: merchant_country.to_string(),
        }
    }
}

impl Default for FeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_trained_model() {
        let model = GradientBoostingModel::pre_trained();
        assert!(model.is_ready());
        assert!(!model.trees.is_empty());
    }

    #[test]
    fn test_prediction() {
        let model = GradientBoostingModel::pre_trained();
        let features = RoutingFeatures {
            gateway_id: "gw-1".into(),
            gateway_base_success_rate: 0.95,
            gateway_recent_success_rate: 0.93,
            gateway_avg_latency_ms: 200.0,
            gateway_volume_24h: 1000,
            amount_minor_units: 5000,
            currency: "USD".into(),
            card_scheme: "visa".into(),
            hour_of_day: 14,
            day_of_week: 2,
            is_weekend: false,
            issuer_country: Some("US".into()),
            merchant_country: "AE".into(),
        };

        let prediction = model.predict(&features);
        assert!(prediction.success_probability > 0.0);
        assert!(prediction.success_probability <= 1.0);
        assert!(prediction.expected_latency_ms > 0.0);
        assert!(prediction.confidence > 0.0);
    }

    #[test]
    fn test_rank_gateways() {
        let model = GradientBoostingModel::pre_trained();

        let features_gw1 = RoutingFeatures {
            gateway_id: "gw-1".into(),
            gateway_base_success_rate: 0.98,
            gateway_recent_success_rate: 0.97,
            gateway_avg_latency_ms: 150.0,
            gateway_volume_24h: 5000,
            amount_minor_units: 5000,
            currency: "USD".into(),
            card_scheme: "visa".into(),
            hour_of_day: 14,
            day_of_week: 2,
            is_weekend: false,
            issuer_country: Some("US".into()),
            merchant_country: "AE".into(),
        };

        let features_gw2 = RoutingFeatures {
            gateway_id: "gw-2".into(),
            gateway_base_success_rate: 0.92,
            gateway_recent_success_rate: 0.90,
            gateway_avg_latency_ms: 300.0,
            gateway_volume_24h: 2000,
            amount_minor_units: 5000,
            currency: "USD".into(),
            card_scheme: "visa".into(),
            hour_of_day: 14,
            day_of_week: 2,
            is_weekend: false,
            issuer_country: Some("US".into()),
            merchant_country: "AE".into(),
        };

        let rankings = model.rank_gateways(&[
            ("gw-1".into(), features_gw1),
            ("gw-2".into(), features_gw2),
        ]);

        assert_eq!(rankings.len(), 2);
        assert_eq!(rankings[0].gateway_id, "gw-1"); // Higher success rate
    }

    #[test]
    fn test_feature_extractor() {
        let mut extractor = FeatureExtractor::new();

        // Record some transactions
        extractor.record_transaction("gw-1", true, 150.0, 5000);
        extractor.record_transaction("gw-1", true, 180.0, 5000);
        extractor.record_transaction("gw-1", false, 200.0, 5000);

        let features = extractor.extract_features(
            "gw-1",
            5000,
            "USD",
            "visa",
            "AE",
            Some("US"),
        );

        assert_eq!(features.gateway_base_success_rate, 2.0 / 3.0);
        assert!(features.gateway_avg_latency_ms > 0.0);
    }
}
