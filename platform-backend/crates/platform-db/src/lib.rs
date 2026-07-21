#![allow(dead_code, unused_imports)]
//! Shared database and cache connection utilities with retry logic.
//! Used by all services — eliminates code duplication.

pub mod health;
pub mod leader;

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

/// Set the RLS tenant context for the current database session.
///
/// Must be called at the start of each request to enforce Row-Level Security (SRS SECTEST-004).
/// Uses `SET LOCAL` so the setting applies only to the current transaction.
///
/// ```rust,no_run
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
/// let operator_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
/// platform_db::set_operator_context(&db, operator_id).await?;
/// // All subsequent queries in this transaction are filtered by operator_id
/// # Ok(())
/// # }
/// ```
pub async fn set_operator_context(
    db: &DatabaseConnection,
    operator_id: uuid::Uuid,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::{ConnectionTrait, Statement};

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT set_config('app.current_operator_id', $1, true)",
        vec![operator_id.to_string().into()],
    ))
    .await?;

    Ok(())
}

/// Clear the RLS tenant context (for system-level operations).
pub async fn clear_operator_context(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    use sea_orm::{ConnectionTrait, Statement};

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT set_config('app.current_operator_id', '', true)",
        vec![],
    ))
    .await?;

    Ok(())
}
