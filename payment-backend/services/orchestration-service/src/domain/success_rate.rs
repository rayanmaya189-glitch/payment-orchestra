//! Success rate tracking and routing for payment gateways.
//!
//! This module provides:
//! - Success rate calculation per gateway
//! - Sliding window statistics
//! - Success rate-based routing decisions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Success rate tracker for a single gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySuccessRate {
    pub gateway_id: Uuid,
    pub total_attempts: u64,
    pub successful_attempts: u64,
    pub failed_attempts: u64,
    pub last_updated: DateTime<Utc>,
    /// Sliding window statistics (last N transactions)
    pub window_stats: SlidingWindowStats,
}

/// Sliding window statistics for recent transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidingWindowStats {
    pub window_size: usize,
    pub recent_results: Vec<bool>,
    pub window_success_count: u64,
    pub window_failure_count: u64,
}

impl SlidingWindowStats {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            recent_results: Vec::with_capacity(window_size),
            window_success_count: 0,
            window_failure_count: 0,
        }
    }

    pub fn record(&mut self, success: bool) {
        if self.recent_results.len() >= self.window_size {
            if let Some(oldest) = self.recent_results.first().cloned() {
                if oldest {
                    self.window_success_count = self.window_success_count.saturating_sub(1);
                } else {
                    self.window_failure_count = self.window_failure_count.saturating_sub(1);
                }
                self.recent_results.remove(0);
            }
        }
        self.recent_results.push(success);
        if success {
            self.window_success_count += 1;
        } else {
            self.window_failure_count += 1;
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.window_success_count + self.window_failure_count;
        if total == 0 {
            return 1.0; // Default to 100% if no data
        }
        self.window_success_count as f64 / total as f64
    }
}

impl GatewaySuccessRate {
    pub fn new(gateway_id: Uuid) -> Self {
        Self {
            gateway_id,
            total_attempts: 0,
            successful_attempts: 0,
            failed_attempts: 0,
            last_updated: Utc::now(),
            window_stats: SlidingWindowStats::new(100),
        }
    }

    pub fn record_success(&mut self) {
        self.total_attempts += 1;
        self.successful_attempts += 1;
        self.last_updated = Utc::now();
        self.window_stats.record(true);
    }

    pub fn record_failure(&mut self) {
        self.total_attempts += 1;
        self.failed_attempts += 1;
        self.last_updated = Utc::now();
        self.window_stats.record(false);
    }

    /// Get overall success rate (all-time).
    pub fn overall_success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            return 1.0; // Default to 100% if no data
        }
        self.successful_attempts as f64 / self.total_attempts as f64
    }

    /// Get windowed success rate (recent transactions).
    pub fn windowed_success_rate(&self) -> f64 {
        self.window_stats.success_rate()
    }

    /// Get composite success rate (weighted combination).
    pub fn composite_success_rate(&self) -> f64 {
        let overall = self.overall_success_rate();
        let windowed = self.windowed_success_rate();
        // Weight windowed more heavily for recency
        0.3 * overall + 0.7 * windowed
    }
}

/// Global success rate tracker for all gateways.
#[derive(Debug, Clone)]
pub struct SuccessRateTracker {
    /// Gateway success rates indexed by gateway_id
    rates: HashMap<Uuid, GatewaySuccessRate>,
    /// Minimum attempts before considering success rate
    min_attempts_threshold: u64,
    /// Default success rate for new gateways
    default_success_rate: f64,
}

impl SuccessRateTracker {
    pub fn new() -> Self {
        Self {
            rates: HashMap::new(),
            min_attempts_threshold: 10,
            default_success_rate: 0.95,
        }
    }

    pub fn with_threshold(mut self, threshold: u64) -> Self {
        self.min_attempts_threshold = threshold;
        self
    }

    pub fn with_default_rate(mut self, rate: f64) -> Self {
        self.default_success_rate = rate;
        self
    }

    /// Record a successful transaction for a gateway.
    pub fn record_success(&mut self, gateway_id: Uuid) {
        let rate = self
            .rates
            .entry(gateway_id)
            .or_insert_with(|| GatewaySuccessRate::new(gateway_id));
        rate.record_success();
    }

    /// Record a failed transaction for a gateway.
    pub fn record_failure(&mut self, gateway_id: Uuid) {
        let rate = self
            .rates
            .entry(gateway_id)
            .or_insert_with(|| GatewaySuccessRate::new(gateway_id));
        rate.record_failure();
    }

    /// Get the composite success rate for a gateway.
    pub fn get_success_rate(&self, gateway_id: &Uuid) -> f64 {
        if let Some(rate) = self.rates.get(gateway_id) {
            if rate.total_attempts >= self.min_attempts_threshold {
                return rate.composite_success_rate();
            }
        }
        self.default_success_rate
    }

    /// Get all gateway success rates.
    pub fn get_all_rates(&self) -> &HashMap<Uuid, GatewaySuccessRate> {
        &self.rates
    }

    /// Sort gateways by success rate (descending).
    pub fn rank_by_success_rate(&self, gateway_ids: &[Uuid]) -> Vec<(Uuid, f64)> {
        let mut ranked: Vec<(Uuid, f64)> = gateway_ids
            .iter()
            .map(|id| (*id, self.get_success_rate(id)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Select the best gateway based on success rate.
    pub fn select_best_gateway(&self, available_gateways: &[Uuid]) -> Option<Uuid> {
        if available_gateways.is_empty() {
            return None;
        }
        let ranked = self.rank_by_success_rate(available_gateways);
        ranked.first().map(|(id, _)| *id)
    }
}

impl Default for SuccessRateTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_stats() {
        let mut stats = SlidingWindowStats::new(5);
        
        // Record 3 successes
        stats.record(true);
        stats.record(true);
        stats.record(true);
        assert_eq!(stats.window_success_count, 3);
        assert_eq!(stats.window_failure_count, 0);
        assert!((stats.success_rate() - 1.0).abs() < f64::EPSILON);
        
        // Record 2 failures
        stats.record(false);
        stats.record(false);
        assert_eq!(stats.window_success_count, 3);
        assert_eq!(stats.window_failure_count, 2);
        assert!((stats.success_rate() - 0.6).abs() < f64::EPSILON);
        
        // Record 1 more success (should push out oldest success)
        // Window before: [true, true, true, false, false]
        // Window after:  [true, true, false, false, true]
        stats.record(true);
        assert_eq!(stats.window_success_count, 3);
        assert_eq!(stats.window_failure_count, 2);
    }

    #[test]
    fn test_gateway_success_rate() {
        let mut rate = GatewaySuccessRate::new(Uuid::now_v7());
        
        // Record some successes and failures
        for _ in 0..8 {
            rate.record_success();
        }
        for _ in 0..2 {
            rate.record_failure();
        }
        
        assert_eq!(rate.overall_success_rate(), 0.8);
        assert_eq!(rate.total_attempts, 10);
    }

    #[test]
    fn test_success_rate_tracker() {
        let mut tracker = SuccessRateTracker::new().with_threshold(5);
        let gw1 = Uuid::now_v7();
        let gw2 = Uuid::now_v7();
        
        // Record successes for gw1
        for _ in 0..9 {
            tracker.record_success(gw1);
        }
        tracker.record_failure(gw1);
        
        // Record failures for gw2
        for _ in 0..7 {
            tracker.record_failure(gw2);
        }
        tracker.record_success(gw2);
        tracker.record_success(gw2);
        
        // gw1 should have higher success rate
        let rate1 = tracker.get_success_rate(&gw1);
        let rate2 = tracker.get_success_rate(&gw2);
        assert!(rate1 > rate2);
        
        // Best gateway should be gw1
        let best = tracker.select_best_gateway(&[gw1, gw2]);
        assert_eq!(best, Some(gw1));
    }

    #[test]
    fn test_rank_by_success_rate() {
        let mut tracker = SuccessRateTracker::new().with_threshold(5);
        let gw1 = Uuid::now_v7();
        let gw2 = Uuid::now_v7();
        let gw3 = Uuid::now_v7();
        
        // gw1: 100% success
        for _ in 0..10 {
            tracker.record_success(gw1);
        }
        
        // gw2: 80% success
        for _ in 0..8 {
            tracker.record_success(gw2);
        }
        for _ in 0..2 {
            tracker.record_failure(gw2);
        }
        
        // gw3: 60% success
        for _ in 0..6 {
            tracker.record_success(gw3);
        }
        for _ in 0..4 {
            tracker.record_failure(gw3);
        }
        
        let ranked = tracker.rank_by_success_rate(&[gw1, gw2, gw3]);
        assert_eq!(ranked[0].0, gw1);
        assert_eq!(ranked[1].0, gw2);
        assert_eq!(ranked[2].0, gw3);
    }
}
