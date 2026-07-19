use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub service: String,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub actor_type: Option<String>,
    pub event_type: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

pub struct ServiceLogger {
    service_name: String,
}

impl ServiceLogger {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    pub fn init(service_name: &str) {
        let filter = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| format!("{service_name}=info,tower_http=debug"));

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .json()
            .init();

        info!(service = service_name, "Service logger initialized");
    }

    pub fn structured(
    #[allow(clippy::too_many_arguments)]
        &self,
        level: &str,
        correlation_id: Option<Uuid>,
        actor_id: Option<Uuid>,
        actor_type: Option<&str>,
        event_type: Option<&str>,
        message: &str,
        metadata: Option<serde_json::Value>,
    ) {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            service: self.service_name.clone(),
            correlation_id,
            causation_id: None,
            actor_id,
            actor_type: actor_type.map(String::from),
            event_type: event_type.map(String::from),
            message: message.to_string(),
            metadata,
        };

        if let Ok(json) = serde_json::to_string(&entry) {
            match level {
                "ERROR" => tracing::error!("{}", json),
                "WARN" => tracing::warn!("{}", json),
                "DEBUG" => tracing::debug!("{}", json),
                _ => tracing::info!("{}", json),
            }
        }
    }
}
