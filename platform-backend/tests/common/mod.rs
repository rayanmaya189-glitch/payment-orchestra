//! Shared test utilities for integration tests.
//!
//! Provides helpers for setting up test databases, mock services,
//! and test fixtures.

use uuid::Uuid;

/// Generate a test UUID.
pub fn test_uuid() -> Uuid {
    Uuid::now_v7()
}

/// Create a test Money value.
pub fn test_money(amount: i64, currency: &str) -> shared_types::Money {
    shared_types::Money {
        amount_minor_units: amount,
        currency: shared_types::CurrencyCode::new(currency).unwrap(),
    }
}

/// Create a test AED Money value.
pub fn test_aed(amount: i64) -> shared_types::Money {
    test_money(amount, "AED")
}

/// Test environment configuration.
pub struct TestConfig {
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: String,
}

impl TestConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/platform_test".to_string()),
            redis_url: std::env::var("TEST_REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            nats_url: std::env::var("TEST_NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation() {
        let id = test_uuid();
        assert!(!id.is_nil());
    }

    #[test]
    fn test_money_creation() {
        let money = test_aed(10000);
        assert_eq!(money.amount_minor_units, 10000);
        assert_eq!(money.currency.0, "AED");
    }
}
