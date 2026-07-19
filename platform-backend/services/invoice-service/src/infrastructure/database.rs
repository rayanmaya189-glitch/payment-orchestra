use sea_orm::{Database, DatabaseConnection};
use platform_config::DatabaseConfig;

pub async fn connect(config: &DatabaseConfig) -> DatabaseConnection {
    Database::connect(&config.url)
        .await
        .expect("Failed to connect to database")
}
