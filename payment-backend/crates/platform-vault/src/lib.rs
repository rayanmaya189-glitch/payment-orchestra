//! HashiCorp Vault integration for secrets management.
//!
//! This crate provides:
//! - KV v2 secrets engine support
//! - AppRole and Token authentication
//! - Role-based secret access control
//! - Automatic secret caching
//! - Policy management

pub mod client;
pub mod secrets;
pub mod policies;

pub use client::{VaultClient, VaultConfig, AuthMethod, VaultHealth};
pub use secrets::{SecretsManager, SecretRole};
pub use policies::{VaultPolicy, PlatformPolicies};

/// Vault error types
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Client error: {0}")]
    ClientError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Lock error: {0}")]
    LockError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
