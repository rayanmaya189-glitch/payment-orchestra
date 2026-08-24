//! Success rate tracking and routing for payment gateways.
//!
//! This module provides:
//! - Success rate calculation per gateway
//! - Sliding window statistics
//! - Success rate-based routing decisions
//! - Redis-backed persistence for distributed deployments

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use redis::aio::ConnectionManager;
use tracing::{warn, info};

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

// ─── Redis-Backed Success Rate Tracker ──────────────────────────────────────

/// Redis-backed success rate tracker for distributed deployments.
///
/// Uses Redis sorted sets for sliding window statistics and hash maps
/// for aggregate counts. Provides the same interface as [`SuccessRateTracker`]
/// but persists data across restarts and shares state across instances.
#[derive(Clone)]
pub struct RedisSuccessRateTracker {
    redis: ConnectionManager,
    window_size: usize,
    min_attempts_threshold: u64,
    default_success_rate: f64,
}

impl RedisSuccessRateTracker {
    /// Create a new Redis-backed success rate tracker.
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis,
            window_size: 100,
            min_attempts_threshold: 10,
            default_success_rate: 0.95,
        }
    }

    /// Set the sliding window size.
    pub fn with_window_size(mut self, size: usize) -> Self {
        self.window_size = size;
        self
    }

    /// Set the minimum attempts threshold.
    pub fn with_threshold(mut self, threshold: u64) -> Self {
        self.min_attempts_threshold = threshold;
        self
    }

    /// Set the default success rate for new gateways.
    pub fn with_default_rate(mut self, rate: f64) -> Self {
        self.default_success_rate = rate;
        self
    }

    /// Record a successful transaction for a gateway.
    pub async fn record_success(&self, gateway_id: Uuid) {
        let gw_key = format!("sr:gw:{}", gateway_id);
        let window_key = format!("sr:window:{}", gateway_id);
        let now_ms = Utc::now().timestamp_millis();

        // Increment success count and total attempts
        let mut pipeline = redis::pipe();
        pipeline
            .hincr(&gw_key, "successes", 1)
            .hincr(&gw_key, "total", 1)
            .hset(&gw_key, "updated_at", now_ms)
            .zadd(&window_key, format!("1:{}", now_ms), now_ms)
            .ignore();

        if let Err(e) = pipeline
            .query_async::<()>(&mut self.redis.clone())
            .await
        {
            warn!(gateway_id = %gateway_id, error = %e, "Redis record_success failed");
            return;
        }

        // Trim window to keep only last N entries
        self.trim_window(&window_key).await;
    }

    /// Record a failed transaction for a gateway.
    pub async fn record_failure(&self, gateway_id: Uuid) {
        let gw_key = format!("sr:gw:{}", gateway_id);
        let window_key = format!("sr:window:{}", gateway_id);
        let now_ms = Utc::now().timestamp_millis();

        // Increment failure count and total attempts
        let mut pipeline = redis::pipe();
        pipeline
            .hincr(&gw_key, "failures", 1)
            .hincr(&gw_key, "total", 1)
            .hset(&gw_key, "updated_at", now_ms)
            .zadd(&window_key, format!("0:{}", now_ms), now_ms)
            .ignore();

        if let Err(e) = pipeline
            .query_async::<()>(&mut self.redis.clone())
            .await
        {
            warn!(gateway_id = %gateway_id, error = %e, "Redis record_failure failed");
            return;
        }

        // Trim window to keep only last N entries
        self.trim_window(&window_key).await;
    }

    /// Get the composite success rate for a gateway.
    pub async fn get_success_rate(&self, gateway_id: &Uuid) -> f64 {
        let gw_key = format!("sr:gw:{}", gateway_id);
        let window_key = format!("sr:window:{}", gateway_id);

        // Get aggregate counts
        let total: u64 = redis::cmd("HGET")
            .arg(&gw_key)
            .arg("total")
            .query_async(&mut self.redis.clone())
            .await
            .unwrap_or(0);

        // Check if we have enough data
        if total < self.min_attempts_threshold {
            return self.default_success_rate;
        }

        // Get windowed success count
        let window_successes = self.count_window_successes(&window_key).await;
        let window_total = self.window_size as f64;

        if window_total == 0.0 {
            return self.default_success_rate;
        }

        // Composite rate: 30% overall + 70% windowed
        let overall_rate = if total > 0 {
            let successes: u64 = redis::cmd("HGET")
                .arg(&gw_key)
                .arg("successes")
                .query_async(&mut self.redis.clone())
                .await
                .unwrap_or(0);
            successes as f64 / total as f64
        } else {
            1.0
        };

        let windowed_rate = window_successes as f64 / window_total;
        0.3 * overall_rate + 0.7 * windowed_rate
    }

    /// Rank gateways by success rate (descending).
    pub async fn rank_by_success_rate(&self, gateway_ids: &[Uuid]) -> Vec<(Uuid, f64)> {
        let mut ranked: Vec<(Uuid, f64)> = Vec::with_capacity(gateway_ids.len());
        for id in gateway_ids {
            let rate = self.get_success_rate(id).await;
            ranked.push((*id, rate));
        }
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Select the best gateway based on success rate.
    pub async fn select_best_gateway(&self, available_gateways: &[Uuid]) -> Option<Uuid> {
        if available_gateways.is_empty() {
            return None;
        }
        let ranked = self.rank_by_success_rate(available_gateways).await;
        ranked.first().map(|(id, _)| *id)
    }

    /// Trim the sliding window to keep only the last N entries.
    async fn trim_window(&self, window_key: &str) {
        let count: u64 = redis::cmd("ZCARD")
            .arg(window_key)
            .query_async(&mut self.redis.clone())
            .await
            .unwrap_or(0);

        if count > self.window_size as u64 {
            // Remove oldest entries beyond window size
            let _: Result<(), _> = redis::cmd("ZREMRANGEBYRANK")
                .arg(window_key)
                .arg(0)
                .arg(-(self.window_size as i64 + 1))
                .query_async(&mut self.redis.clone())
                .await;
        }
    }

    /// Count successful transactions in the sliding window.
    async fn count_window_successes(&self, window_key: &str) -> u64 {
        // Get all members and count those starting with "1:" (success)
        let members: Vec<String> = redis::cmd("ZRANGE")
            .arg(window_key)
            .arg(0)
            .arg(-1)
            .query_async(&mut self.redis.clone())
            .await
            .unwrap_or_default();

        members.iter().filter(|m| m.starts_with("1:")).count() as u64
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
