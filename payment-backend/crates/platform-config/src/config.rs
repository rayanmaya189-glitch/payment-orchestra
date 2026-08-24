use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    // Database
    pub database_url: String,
    pub database_pool_size: u32,
    pub database_max_lifetime_secs: u64,
    // Redis
    pub redis_url: String,
    pub redis_pool_size: u32,
    // Service identity
    pub service_name: String,
    pub listen_addr: String,
    pub health_listen_addr: String,
    pub graceful_shutdown_timeout_secs: u64,
    // Auth
    pub jwt_secret: String,
    pub kms_kek_id: String,
    // Logging
    pub log_level: String,
    // Etcd service discovery
    pub etcd_endpoints: Vec<String>,
    pub etcd_lease_ttl_secs: i64,
    // Service gRPC port (numeric)
    pub grpc_port: u16,
    // Service health port (numeric)
    pub health_port: u16,
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
            etcd_endpoints: vec!["http://localhost:2379".into()],
            etcd_lease_ttl_secs: 30,
            grpc_port: 9000,
            health_port: 9100,
        }
    }
}
