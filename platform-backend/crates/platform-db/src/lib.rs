#![allow(dead_code, unused_imports)]
//! Shared database and cache connection utilities with retry logic.
//! Used by all services — eliminates code duplication.

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use redis::aio::ConnectionManager;
use redis::Client;
use platform_config::{DatabaseConfig, RedisConfig};
use tracing::{info, warn};

/// Connect to PostgreSQL with exponential-backoff retry and connection pooling.
///
/// - Retries up to 10 times with exponential backoff (100ms, 200ms, 400ms, ...).
/// - Configures min/max connections, acquire timeout, idle timeout, and max lifetime.
/// - Panics after exhausting retries (acceptable at startup).
pub async fn connect_database(config: &DatabaseConfig) -> DatabaseConnection {
    let max_retries = 10u32;
    let mut attempt = 0u32;

    loop {
        attempt += 1;
        let mut options = ConnectOptions::new(config.url.as_str());
        options
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .max_lifetime(std::time::Duration::from_secs(config.max_lifetime_secs))
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout_secs))
            .idle_timeout(std::time::Duration::from_secs(300));

        match Database::connect(options).await {
            Ok(conn) => {
                info!(
                    "PostgreSQL connected (pool={})",
                    config.max_connections
                );
                return conn;
            }
            Err(e) => {
                if attempt >= max_retries {
                    panic!(
                        "PostgreSQL connection failed after {} attempts: {}",
                        max_retries, e
                    );
                }
                let backoff_ms = 2u64.pow(attempt - 1) * 100;
                warn!(
                    "PostgreSQL attempt {}/{} failed: {}. Retrying in {}ms...",
                    attempt, max_retries, e, backoff_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}

/// Connect to Redis with exponential-backoff retry.
///
/// Creates the client once, then retries only the connection manager.
/// Panics after exhausting retries (acceptable at startup).
pub async fn connect_redis(config: &RedisConfig) -> ConnectionManager {
    let max_retries = 10u32;
    let mut attempt = 0u32;

    let client = Client::open(config.url.as_str())
        .expect("Failed to create Redis client — check REDIS_URL");

    loop {
        attempt += 1;
        match client.get_connection_manager().await {
            Ok(conn) => {
                info!("Redis connected");
                return conn;
            }
            Err(e) => {
                if attempt >= max_retries {
                    panic!(
                        "Redis connection failed after {} attempts: {}",
                        max_retries, e
                    );
                }
                let backoff_ms = 2u64.pow(attempt - 1) * 100;
                warn!(
                    "Redis attempt {}/{} failed: {}. Retrying in {}ms...",
                    attempt, max_retries, e, backoff_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}
