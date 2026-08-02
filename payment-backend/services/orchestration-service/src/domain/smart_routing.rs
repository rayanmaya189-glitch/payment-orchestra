//! Smart routing engine with ML-based scoring.
//!
//! This module provides:
//! - Machine learning-based routing decisions
//! - Feature extraction for routing
//! - Model inference for gateway selection
//! - A/B testing for routing strategies

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::success_rate::SuccessRateTracker;

/// Features extracted for ML routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingFeatures {
    /// Transaction amount in minor units
    pub amount_minor: i64,
    /// Currency code
    pub currency: String,
    /// Card scheme (visa, mastercard, etc.)
    pub card_scheme: String,
    /// Card issuer country
    pub issuer_country: Option<String>,
    /// Card bin range
    pub card_bin: Option<String>,
    /// Merchant category code
    pub merchant_category_code: Option<String>,
    /// Time of day (hour)
    pub hour_of_day: u32,
    /// Day of week
    pub day_of_week: u32,
    /// Historical success rate for this gateway
    pub gateway_success_rate: f64,
    /// Historical latency for this gateway
    pub gateway_avg_latency_ms: u64,
    /// Gateway health score
    pub gateway_health_score: f64,
    /// Whether this is a retry attempt
    pub is_retry: bool,
    /// Previous gateway (if retry)
    pub previous_gateway: Option<Uuid>,
    /// Risk score (if available)
    pub risk_score: Option<f64>,
}

/// ML routing prediction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPrediction {
    /// Selected gateway ID
    pub gateway_id: Uuid,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Expected success probability
    pub expected_success_rate: f64,
    /// Expected latency (ms)
    pub expected_latency_ms: u64,
    /// Expected cost (if cost-based routing)
    pub expected_cost: Option<f64>,
    /// Routing reason
    pub routing_reason: String,
    /// Alternative gateways with scores
    pub alternatives: Vec<(Uuid, f64)>,
}

/// ML routing model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartRoutingConfig {
    /// Enable ML-based routing
    pub enabled: bool,
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Enable A/B testing
    pub enable_ab_testing: bool,
    /// A/B test percentage (0-100)
    pub ab_test_percentage: u32,
    /// Feature weights for scoring
    pub feature_weights: FeatureWeights,
    /// Model version
    pub model_version: String,
    /// Last model update timestamp
    pub last_model_update: DateTime<Utc>,
}

impl Default for SmartRoutingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_confidence: 0.7,
            enable_ab_testing: false,
            ab_test_percentage: 10,
            feature_weights: FeatureWeights::default(),
            model_version: "1.0.0".into(),
            last_model_update: Utc::now(),
        }
    }
}

/// Feature weights for ML scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureWeights {
    /// Weight for success rate
    pub success_rate_weight: f64,
    /// Weight for latency
    pub latency_weight: f64,
    /// Weight for cost
    pub cost_weight: f64,
    /// Weight for gateway health
    pub health_weight: f64,
    /// Weight for amount matching
    pub amount_weight: f64,
    /// Weight for currency support
    pub currency_weight: f64,
    /// Weight for card scheme support
    pub card_scheme_weight: f64,
}

impl Default for FeatureWeights {
    fn default() -> Self {
        Self {
            success_rate_weight: 0.4,
            latency_weight: 0.2,
            cost_weight: 0.1,
            health_weight: 0.15,
            amount_weight: 0.05,
            currency_weight: 0.05,
            card_scheme_weight: 0.05,
        }
    }
}

/// Smart routing engine.
pub struct SmartRoutingEngine {
    config: SmartRoutingConfig,
    success_rate_tracker: SuccessRateTracker,
}

impl SmartRoutingEngine {
    pub fn new(config: SmartRoutingConfig) -> Self {
        Self {
            config,
            success_rate_tracker: SuccessRateTracker::new(),
        }
    }

    /// Predict the best gateway for a transaction.
    pub fn predict(
        &self,
        features: &RoutingFeatures,
        available_gateways: &[Uuid],
        gateway_features: &[(Uuid, GatewayFeatures)],
    ) -> Option<RoutingPrediction> {
        if !self.config.enabled {
            return None;
        }

        if available_gateways.is_empty() {
            return None;
        }

        // Score each gateway
        let mut scored_gateways: Vec<(Uuid, f64)> = gateway_features
            .iter()
            .filter(|(id, _)| available_gateways.contains(id))
            .map(|(id, gw_features)| {
                let score = self.score_gateway(features, gw_features);
                (*id, score)
            })
            .collect();

        // Sort by score (descending)
        scored_gateways.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if scored_gateways.is_empty() {
            return None;
        }

        let (best_gateway, best_score) = scored_gateways[0];

        // Check confidence threshold
        let confidence = self.calculate_confidence(&scored_gateways);
        if confidence < self.config.min_confidence {
            // Fall back to priority routing
            return None;
        }

        // Calculate expected metrics
        let best_features = gateway_features
            .iter()
            .find(|(id, _)| *id == best_gateway)
            .map(|(_, f)| f)?;

        let expected_success_rate = self.estimate_success_rate(features, best_features);
        let expected_latency_ms = self.estimate_latency(features, best_features);

        Some(RoutingPrediction {
            gateway_id: best_gateway,
            confidence,
            expected_success_rate,
            expected_latency_ms,
            expected_cost: self.estimate_cost(features, best_features),
            routing_reason: self.generate_routing_reason(best_score, expected_success_rate),
            alternatives: scored_gateways[1..].to_vec().into_iter().take(3).collect(),
        })
    }

    /// Score a gateway based on features.
    fn score_gateway(&self, features: &RoutingFeatures, gw_features: &GatewayFeatures) -> f64 {
        let weights = &self.config.feature_weights;

        // Success rate score (normalized 0-1)
        let success_score = gw_features.success_rate;

        // Latency score (lower is better, normalize to 0-1)
        let latency_score = if gw_features.avg_latency_ms > 0 {
            1.0 - (gw_features.avg_latency_ms as f64 / 5000.0).min(1.0)
        } else {
            1.0
        };

        // Cost score (lower is better, normalize to 0-1)
        let cost_score = if let Some(cost) = gw_features.transaction_fee_bps {
            1.0 - (cost as f64 / 500.0).min(1.0) // 500 bps = max cost
        } else {
            0.5 // Default if no cost info
        };

        // Health score (already 0-1)
        let health_score = gw_features.health_score;

        // Amount match score
        let amount_score = if let Some(max_amount) = gw_features.max_amount_minor {
            if features.amount_minor <= max_amount {
                1.0
            } else {
                0.0
            }
        } else {
            1.0 // No limit
        };

        // Currency support score
        let currency_score = if gw_features.supported_currencies.contains(&features.currency) {
            1.0
        } else {
            0.0
        };

        // Card scheme support score
        let card_scheme_score = if gw_features
            .supported_card_schemes
            .contains(&features.card_scheme)
        {
            1.0
        } else {
            0.0
        };

        // Weighted sum
        let score = weights.success_rate_weight * success_score
            + weights.latency_weight * latency_score
            + weights.cost_weight * cost_score
            + weights.health_weight * health_score
            + weights.amount_weight * amount_score
            + weights.currency_weight * currency_score
            + weights.card_scheme_weight * card_scheme_score;

        score
    }

    /// Calculate confidence based on score distribution.
    fn calculate_confidence(&self, scored_gateways: &[(Uuid, f64)]) -> f64 {
        if scored_gateways.len() < 2 {
            return 1.0;
        }

        let best_score = scored_gateways[0].1;
        let second_best_score = scored_gateways[1].1;

        // Confidence is higher when there's a clear winner
        let score_gap = best_score - second_best_score;
        let confidence = 0.5 + (score_gap * 2.0);
        confidence.min(1.0)
    }

    /// Estimate success rate for a gateway.
    fn estimate_success_rate(
        &self,
        _features: &RoutingFeatures,
        gw_features: &GatewayFeatures,
    ) -> f64 {
        gw_features.success_rate
    }

    /// Estimate latency for a gateway.
    fn estimate_latency(
        &self,
        _features: &RoutingFeatures,
        gw_features: &GatewayFeatures,
    ) -> u64 {
        gw_features.avg_latency_ms
    }

    /// Estimate cost for a gateway.
    fn estimate_cost(
        &self,
        features: &RoutingFeatures,
        gw_features: &GatewayFeatures,
    ) -> Option<f64> {
        gw_features.transaction_fee_bps.map(|bps| {
            let fee = features.amount_minor as f64 * bps as f64 / 10000.0;
            fee
        })
    }

    /// Generate routing reason.
    fn generate_routing_reason(&self, score: f64, success_rate: f64) -> String {
        if success_rate > 0.95 {
            "High success rate gateway selected".to_string()
        } else if score > 0.8 {
            "Best overall match based on features".to_string()
        } else {
            "ML-based routing selection".to_string()
        }
    }

    /// Record transaction outcome for learning.
    pub fn record_outcome(&mut self, gateway_id: Uuid, success: bool) {
        if success {
            self.success_rate_tracker.record_success(gateway_id);
        } else {
            self.success_rate_tracker.record_failure(gateway_id);
        }
    }
}

/// Gateway-specific features for ML scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayFeatures {
    /// Current success rate
    pub success_rate: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: u64,
    /// Health score (0.0 - 1.0)
    pub health_score: f64,
    /// Transaction fee in basis points
    pub transaction_fee_bps: Option<u32>,
    /// Maximum transaction amount
    pub max_amount_minor: Option<i64>,
    /// Supported currencies
    pub supported_currencies: Vec<String>,
    /// Supported card schemes
    pub supported_card_schemes: Vec<String>,
    /// Is gateway currently available
    pub is_available: bool,
    /// Circuit breaker state
    pub circuit_breaker_state: CircuitBreakerState,
}

/// Circuit breaker state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreakerState {
    pub fn is_usable(&self) -> bool {
        matches!(self, CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_routing_disabled() {
        let config = SmartRoutingConfig {
            enabled: false,
            ..Default::default()
        };
        let engine = SmartRoutingEngine::new(config);

        let features = RoutingFeatures {
            amount_minor: 10000,
            currency: "USD".into(),
            card_scheme: "visa".into(),
            issuer_country: Some("US".into()),
            card_bin: Some("411111".into()),
            merchant_category_code: Some("5411".into()),
            hour_of_day: 14,
            day_of_week: 1,
            gateway_success_rate: 0.95,
            gateway_avg_latency_ms: 200,
            gateway_health_score: 0.9,
            is_retry: false,
            previous_gateway: None,
            risk_score: None,
        };

        let gw_id = Uuid::now_v7();
        let gw_features = GatewayFeatures {
            success_rate: 0.95,
            avg_latency_ms: 200,
            health_score: 0.9,
            transaction_fee_bps: Some(150),
            max_amount_minor: Some(1000000),
            supported_currencies: vec!["USD".into()],
            supported_card_schemes: vec!["visa".into()],
            is_available: true,
            circuit_breaker_state: CircuitBreakerState::Closed,
        };

        let result = engine.predict(&features, &[gw_id], &[(gw_id, gw_features)]);
        assert!(result.is_none());
    }

    #[test]
    fn test_smart_routing_enabled() {
        let config = SmartRoutingConfig {
            enabled: true,
            min_confidence: 0.5,
            ..Default::default()
        };
        let engine = SmartRoutingEngine::new(config);

        let features = RoutingFeatures {
            amount_minor: 10000,
            currency: "USD".into(),
            card_scheme: "visa".into(),
            issuer_country: Some("US".into()),
            card_bin: Some("411111".into()),
            merchant_category_code: Some("5411".into()),
            hour_of_day: 14,
            day_of_week: 1,
            gateway_success_rate: 0.95,
            gateway_avg_latency_ms: 200,
            gateway_health_score: 0.9,
            is_retry: false,
            previous_gateway: None,
            risk_score: None,
        };

        let gw1_id = Uuid::now_v7();
        let gw2_id = Uuid::now_v7();

        let gw1_features = GatewayFeatures {
            success_rate: 0.95,
            avg_latency_ms: 150,
            health_score: 0.9,
            transaction_fee_bps: Some(150),
            max_amount_minor: Some(1000000),
            supported_currencies: vec!["USD".into()],
            supported_card_schemes: vec!["visa".into()],
            is_available: true,
            circuit_breaker_state: CircuitBreakerState::Closed,
        };

        let gw2_features = GatewayFeatures {
            success_rate: 0.85,
            avg_latency_ms: 250,
            health_score: 0.7,
            transaction_fee_bps: Some(200),
            max_amount_minor: Some(1000000),
            supported_currencies: vec!["USD".into()],
            supported_card_schemes: vec!["visa".into()],
            is_available: true,
            circuit_breaker_state: CircuitBreakerState::Closed,
        };

        let result = engine.predict(
            &features,
            &[gw1_id, gw2_id],
            &[(gw1_id, gw1_features), (gw2_id, gw2_features)],
        );

        assert!(result.is_some());
        let prediction = result.unwrap();
        assert_eq!(prediction.gateway_id, gw1_id);
        assert!(prediction.confidence > 0.5);
        assert!(prediction.expected_success_rate > 0.8);
    }

    #[test]
    fn test_circuit_breaker_usable() {
        assert!(CircuitBreakerState::Closed.is_usable());
        assert!(CircuitBreakerState::HalfOpen.is_usable());
        assert!(!CircuitBreakerState::Open.is_usable());
    }
}
