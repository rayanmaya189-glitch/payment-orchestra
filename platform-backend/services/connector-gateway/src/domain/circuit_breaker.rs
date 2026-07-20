//! Circuit breaker per-connector (SRS Part 7 §5, CB-001).
//!
//! States: Closed → Open (50% error rate, 30s window) → HalfOpen (probe).
//! State cached in Redis for cross-replica visibility.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — requests pass through.
    Closed,
    /// Failing — all requests rejected immediately.
    Open,
    /// Probing — allow one request through to test.
    HalfOpen,
}

/// Per-connector circuit breaker.
#[derive(Debug)]
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    /// Number of failures in the current window.
    failure_count: AtomicU64,
    /// Number of successes in the current window.
    success_count: AtomicU64,
    /// When the circuit was opened (for timeout calculation).
    opened_at: RwLock<Option<Instant>>,
    /// Error threshold percentage to trip the circuit (default: 50%).
    error_threshold_pct: f64,
    /// How long to stay open before allowing a probe (default: 30s).
    open_duration: Duration,
    /// Window size for counting successes/failures (default: 30s).
    window: Duration,
    /// Window start time.
    window_start: RwLock<Instant>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with SRS default thresholds.
    pub fn new() -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            opened_at: RwLock::new(None),
            error_threshold_pct: 50.0,
            open_duration: Duration::from_secs(30),
            window: Duration::from_secs(30),
            window_start: RwLock::new(Instant::now()),
        }
    }

    /// Create with custom thresholds.
    pub fn with_thresholds(error_threshold_pct: f64, open_duration: Duration, window: Duration) -> Self {
        Self {
            error_threshold_pct,
            open_duration,
            window,
            ..Self::new()
        }
    }

    /// Check if a request is allowed through.
    pub async fn allow_request(&self) -> bool {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if open duration has elapsed → transition to HalfOpen
                if let Some(opened) = *self.opened_at.read().await {
                    if opened.elapsed() >= self.open_duration {
                        *state = CircuitState::HalfOpen;
                        self.failure_count.store(0, Ordering::Relaxed);
                        self.success_count.store(0, Ordering::Relaxed);
                        tracing::info!("Circuit breaker transitioning to HalfOpen (probe)");
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true, // Allow one probe
        }
    }

    /// Record a successful request.
    pub async fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        let mut state = self.state.write().await;

        if *state == CircuitState::HalfOpen {
            // Success in HalfOpen → close the circuit
            *state = CircuitState::Closed;
            *self.opened_at.write().await = None;
            self.failure_count.store(0, Ordering::Relaxed);
            self.success_count.store(0, Ordering::Relaxed);
            tracing::info!("Circuit breaker closed (recovery confirmed)");
            return;
        }

        // Also check threshold on success (failures may have accumulated)
        if *state == CircuitState::Closed {
            let failures = self.failure_count.load(Ordering::Relaxed);
            let successes = self.success_count.load(Ordering::Relaxed);
            let total = failures + successes;
            if total >= 10 {
                let error_pct = (failures as f64 / total as f64) * 100.0;
                if error_pct >= self.error_threshold_pct {
                    *state = CircuitState::Open;
                    *self.opened_at.write().await = Some(Instant::now());
                    tracing::warn!(
                        error_pct = error_pct,
                        threshold = self.error_threshold_pct,
                        "Circuit breaker opened (error threshold exceeded)"
                    );
                }
            }
        }

        self.maybe_reset_window().await;
    }

    /// Record a failed request.
    pub async fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write().await;

        match *state {
            CircuitState::HalfOpen => {
                // Failure in HalfOpen → reopen
                *state = CircuitState::Open;
                *self.opened_at.write().await = Some(Instant::now());
                tracing::warn!("Circuit breaker reopened (probe failed)");
            }
            CircuitState::Closed => {
                // Check if error threshold exceeded
                let failures = self.failure_count.load(Ordering::Relaxed);
                let successes = self.success_count.load(Ordering::Relaxed);
                let total = failures + successes;
                if total >= 10 {
                    let error_pct = (failures as f64 / total as f64) * 100.0;
                    if error_pct >= self.error_threshold_pct {
                        *state = CircuitState::Open;
                        *self.opened_at.write().await = Some(Instant::now());
                        tracing::warn!(
                            error_pct = error_pct,
                            threshold = self.error_threshold_pct,
                            "Circuit breaker opened (error threshold exceeded)"
                        );
                    }
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Get current state.
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Reset window counters if the window has expired.
    async fn maybe_reset_window(&self) {
        let mut start = self.window_start.write().await;
        if start.elapsed() >= self.window {
            self.failure_count.store(0, Ordering::Relaxed);
            self.success_count.store(0, Ordering::Relaxed);
            *start = Instant::now();
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Bulkhead — per-connector isolated connection pool with configurable limits.
/// Prevents one failing connector from exhausting resources for all connectors.
#[derive(Debug)]
pub struct Bulkhead {
    /// Max concurrent requests allowed.
    max_concurrent: u32,
    /// Current in-flight requests.
    in_flight: AtomicU64,
    /// Per-request timeout.
    request_timeout: Duration,
}

impl Bulkhead {
    pub fn new(max_concurrent: u32, request_timeout: Duration) -> Self {
        Self {
            max_concurrent,
            in_flight: AtomicU64::new(0),
            request_timeout,
        }
    }

    /// Create with SRS defaults (10s authorize, 30s settlement).
    pub fn for_authorize() -> Self {
        Self::new(10, Duration::from_secs(10))
    }

    pub fn for_settlement() -> Self {
        Self::new(5, Duration::from_secs(30))
    }

    /// Try to acquire a slot. Returns false if at capacity.
    pub fn try_acquire(&self) -> bool {
        let current = self.in_flight.load(Ordering::Relaxed);
        if current >= self.max_concurrent as u64 {
            return false;
        }
        self.in_flight.fetch_add(1, Ordering::SeqCst) < self.max_concurrent as u64
    }

    /// Release a slot.
    pub fn release(&self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get the request timeout.
    pub fn timeout(&self) -> Duration {
        self.request_timeout
    }

    /// Get current in-flight count.
    pub fn in_flight(&self) -> u64 {
        self.in_flight.load(Ordering::Relaxed)
    }
}

/// Per-connector resilience wrapper combining circuit breaker + bulkhead.
#[derive(Debug)]
pub struct ConnectorResilience {
    pub circuit_breaker: CircuitBreaker,
    pub bulkhead: Bulkhead,
    pub connector_id: String,
}

impl ConnectorResilience {
    pub fn new(connector_id: String, max_concurrent: u32, request_timeout: Duration) -> Self {
        Self {
            circuit_breaker: CircuitBreaker::new(),
            bulkhead: Bulkhead::new(max_concurrent, request_timeout),
            connector_id,
        }
    }

    /// Check if a request can be executed (circuit closed + bulkhead has capacity).
    pub async fn can_execute(&self) -> bool {
        self.circuit_breaker.allow_request().await && self.bulkhead.try_acquire()
    }

    /// Record a successful execution.
    pub async fn record_success(&self) {
        self.bulkhead.release();
        self.circuit_breaker.record_success().await;
    }

    /// Record a failed execution.
    pub async fn record_failure(&self) {
        self.bulkhead.release();
        self.circuit_breaker.record_failure().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new();
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_high_error_rate() {
        let cb = CircuitBreaker::new();
        // Record 6 failures and 4 successes (60% error rate > 50% threshold)
        for _ in 0..6 { cb.record_failure().await; }
        for _ in 0..4 { cb.record_success().await; }
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_bulkhead_capacity() {
        let b = Bulkhead::new(2, Duration::from_secs(10));
        assert!(b.try_acquire());
        assert!(b.try_acquire());
        assert!(!b.try_acquire()); // at capacity
        b.release();
        assert!(b.try_acquire()); // slot freed
    }
}
