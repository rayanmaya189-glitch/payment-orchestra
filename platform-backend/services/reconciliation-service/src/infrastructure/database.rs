use platform_config::DatabaseConfig; use platform_db::connect_database; use sea_orm::DatabaseConnection;
pub async fn connect(config: &DatabaseConfig) -> DatabaseConnection { connect_database(config).await }
