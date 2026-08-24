//! Fraud service configuration

use crate::domain::ModelConfig;

#[derive(Debug, Clone)]
pub struct FraudServiceConfig {
    pub listen_addr: String,
    pub database_url: String,
    pub redis_url: String,
    pub model_config: ModelConfig,
    pub enable_ml_scoring: bool,
    pub log_level: String,
}

impl FraudServiceConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            listen_addr: std::env::var("FRAUD_SERVICE_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:50053".into()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/payment_orchestra".into()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".into()),
            model_config: ModelConfig::default(),
            enable_ml_scoring: std::env::var("ENABLE_ML_SCORING")
                .unwrap_or_else(|_| "true".into())
                .parse()
                .unwrap_or(true),
            log_level: std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".into()),
        })
    }
}
