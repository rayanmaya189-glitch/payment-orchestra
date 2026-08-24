use std::time::Instant;

/// Circuit breaker state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for connector-level fault tolerance.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    error_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    open_duration: std::time::Duration,
    error_threshold: f64,
    #[allow(dead_code)]
    window: std::time::Duration,
    half_open_max_requests: u32,
    half_open_requests: u32,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            error_count: 0,
            success_count: 0,
            last_failure: None,
            open_duration: std::time::Duration::from_secs(60),
            error_threshold: 0.5,
            window: std::time::Duration::from_secs(30),
            half_open_max_requests: 3,
            half_open_requests: 0,
        }
    }

    pub fn state(&self) -> &CircuitState {
        &self.state
    }

    pub fn is_call_allowed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if open duration has elapsed → transition to HalfOpen
                if let Some(last_fail) = self.last_failure {
                    if last_fail.elapsed() >= self.open_duration {
                        self.state = CircuitState::HalfOpen;
                        self.half_open_requests = 0;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                if self.half_open_requests < self.half_open_max_requests {
                    self.half_open_requests += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        match self.state {
            CircuitState::HalfOpen => {
                // Enough successful requests → close the circuit
                self.state = CircuitState::Closed;
                self.error_count = 0;
                self.success_count = 0;
                self.half_open_requests = 0;
            }
            CircuitState::Closed => {
                // Periodically reset counts to avoid stale data
                let total = self.error_count + self.success_count;
                if total > 100 {
                    self.error_count = 0;
                    self.success_count = 0;
                }
            }
            _ => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.error_count += 1;
        self.last_failure = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                let total = self.error_count + self.success_count;
                if total > 0 {
                    let error_rate = self.error_count as f64 / total as f64;
                    if error_rate >= self.error_threshold {
                        self.state = CircuitState::Open;
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Failure in HalfOpen → back to Open
                self.state = CircuitState::Open;
                self.half_open_requests = 0;
            }
            _ => {}
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}
