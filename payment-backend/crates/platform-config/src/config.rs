use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub database_url: String,
    pub database_pool_size: u32,
    pub database_max_lifetime_secs: u64,
    pub redis_url: String,
    pub redis_pool_size: u32,
    pub service_name: String,
    pub listen_addr: String,
    pub health_listen_addr: String,
    pub graceful_shutdown_timeout_secs: u64,
    pub jwt_secret: String,
    pub kms_kek_id: String,
    pub log_level: String,
}

impl ServiceConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://platform:dev_password@localhost:5432/payment_orchestra".into(),
            database_pool_size: 20,
            database_max_lifetime_secs: 1800,
            redis_url: "redis://localhost:6379".into(),
            redis_pool_size: 10,
            service_name: "unknown".into(),
            listen_addr: "0.0.0.0:9000".into(),
            health_listen_addr: "0.0.0.0:8081".into(),
            graceful_shutdown_timeout_secs: 30,
            jwt_secret: "change-me-in-production".into(),
            kms_kek_id: "platform-kek-1".into(),
            log_level: "info".into(),
        }
    }
}
