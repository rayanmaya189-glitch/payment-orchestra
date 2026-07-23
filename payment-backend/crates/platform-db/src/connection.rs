use sea_orm::{Database, DatabaseConnection, DbErr};
use tracing::info;

pub async fn create_postgres_pool(database_url: &str, pool_size: u32) -> Result<DatabaseConnection, DbErr> {
    let mut opt = sea_orm::ConnectOptions::new(database_url);
    opt.max_connections(pool_size)
        .min_connections(5)
        .connect_timeout(std::time::Duration::from_secs(30))
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800));
    Database::connect(opt).await
}

/// Create a database pool using per-service env vars.
/// Reads `{prefix}_DB_HOST`, `{prefix}_DB_PORT`, `{prefix}_DB_NAME`,
/// `{prefix}_DB_USERNAME`, and `{prefix}_DB_PASSWORD` from environment.
pub async fn create_service_pool(prefix: &str) -> Result<DatabaseConnection, String> {
    let host = std::env::var(format!("{}_DB_HOST", prefix)).unwrap_or_else(|_| "localhost".into());
    let port = std::env::var(format!("{}_DB_PORT", prefix)).unwrap_or_else(|_| "5432".into());
    let db_name = std::env::var(format!("{}_DB_NAME", prefix))
        .map_err(|_| format!("{}_DB_NAME not set", prefix))?;
    let username = std::env::var(format!("{}_DB_USERNAME", prefix))
        .map_err(|_| format!("{}_DB_USERNAME not set", prefix))?;
    let password = std::env::var(format!("{}_DB_PASSWORD", prefix))
        .map_err(|_| format!("{}_DB_PASSWORD not set", prefix))?;

    let database_url = format!("postgres://{}:{}@{}:{}/{}", username, password, host, port, db_name);
    info!("Connecting to PostgreSQL at {}:{}/{}", host, port, db_name);
    create_postgres_pool(&database_url, 10).await
        .map_err(|e| format!("PostgreSQL connection failed: {}", e))
}

pub fn create_redis_client(redis_url: &str) -> Result<redis::Client, redis::RedisError> {
    redis::Client::open(redis_url)
}
