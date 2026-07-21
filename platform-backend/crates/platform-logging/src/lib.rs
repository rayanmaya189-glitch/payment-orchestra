#![allow(dead_code, unused_imports)]
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

/// Structured security event for audit logging (SRS AUD-001).
/// These events are logged at WARN/ERROR level for SIEM ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub event_type: SecurityEventType,
    pub principal_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub outcome: SecurityOutcome,
    pub details: Option<serde_json::Value>,
    pub correlation_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    LoginFailed,
    LoginSuccess,
    AccountLocked,
    AccountSuspended,
    SessionRevoked,
    PermissionDenied,
    ApiKeyInvalid,
    RateLimited,
    BruteForceDetected,
    PasswordChanged,
    MfaFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityOutcome {
    Success,
    Failure,
    Blocked,
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

/// Log a structured security event (OWASP A09: Security Logging and Monitoring).
///
/// These events are written as structured JSON at WARN level for SIEM ingestion.
/// Call this for: failed logins, account lockouts, permission denials, rate limiting.
pub fn log_security_event(
    service: &str,
    event_type: SecurityEventType,
    outcome: SecurityOutcome,
    principal_id: Option<Uuid>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    correlation_id: Option<Uuid>,
    details: Option<serde_json::Value>,
) {
    let event = SecurityEvent {
        timestamp: Utc::now(),
        service: service.to_string(),
        event_type,
        principal_id,
        ip_address: ip_address.map(String::from),
        user_agent: user_agent.map(String::from),
        outcome,
        details,
        correlation_id,
    };

    if let Ok(json) = serde_json::to_string(&event) {
        tracing::warn!(security_event = %json, "Security event");
    }
}
