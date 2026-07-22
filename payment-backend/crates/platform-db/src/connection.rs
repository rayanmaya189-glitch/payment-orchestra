use sea_orm::{Database, DatabaseConnection, DbErr};

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

pub fn create_redis_client(redis_url: &str) -> Result<redis::Client, redis::RedisError> {
    redis::Client::open(redis_url)
}
