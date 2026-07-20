use platform_config::RedisConfig; use platform_db::connect_redis; use redis::aio::ConnectionManager;
pub async fn connect(config: &RedisConfig) -> ConnectionManager { connect_redis(config).await }
