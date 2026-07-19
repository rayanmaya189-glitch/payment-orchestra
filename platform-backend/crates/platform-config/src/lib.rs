use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub health_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    pub url: String,
}

impl AppConfig {
    pub fn from_env(service_name: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(Environment::with_prefix(&service_name.to_uppercase()).separator("__"))
            .add_source(Environment::with_prefix("PLATFORM").separator("__"))
            .build()?;

        config.try_deserialize()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                health_port: 8081,
            },
            database: DatabaseConfig {
                url: "postgres://localhost:5432/platform".to_string(),
                max_connections: 10,
                connection_timeout_secs: 5,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
            },
            nats: NatsConfig {
                url: "nats://localhost:4222".to_string(),
            },
        }
    }
}
