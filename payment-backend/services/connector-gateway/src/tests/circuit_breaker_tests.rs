//! Circuit breaker tests.

use crate::domain;

#[test]
fn test_circuit_breaker_initial_state_closed() {
    let mut cb = domain::CircuitBreaker::new();
    assert!(cb.is_call_allowed());
    assert_eq!(*cb.state(), domain::CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_opens_on_high_error_rate() {
    let mut cb = domain::CircuitBreaker::new();
    // Record 6 failures out of 10 requests (60% > 50% threshold)
    for _ in 0..6 {
        cb.record_failure();
    }
    for _ in 0..4 {
        cb.record_success();
    }
    // Check if the circuit opened
    // (max 100 requests before rate check; we have 10)
    let call_allowed = cb.is_call_allowed();
    // After >50% errors, the circuit should be open
    assert!(!call_allowed);
    assert_eq!(*cb.state(), domain::CircuitState::Open);
}

#[test]
fn test_circuit_breaker_records_success_and_failure() {
    let mut cb = domain::CircuitBreaker::new();
    cb.record_success();
    cb.record_failure();
    // State opens after 1 success + 1 failure (50% rate triggers >= 50% threshold)
    assert_eq!(*cb.state(), domain::CircuitState::Open);
}

#[test]
fn test_circuit_breaker_rejects_calls_when_open() {
    let mut cb = domain::CircuitBreaker::new();
    // Force open state
    for _ in 0..100 {
        cb.record_failure();
    }
    assert_eq!(*cb.state(), domain::CircuitState::Open);
    assert!(!cb.is_call_allowed());
}
