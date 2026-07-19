use redis::aio::ConnectionManager; use redis::Client; use platform_config::RedisConfig;
pub async fn connect(config: &RedisConfig) -> ConnectionManager { let client = Client::open(config.url.as_str()).expect("Failed to create Redis client"); client.get_connection_manager().await.expect("Failed to connect to Redis") }
