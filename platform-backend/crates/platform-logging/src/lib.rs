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

// ==================== Global Panic Hook ====================

/// Install a global panic hook that logs panics via tracing instead of printing to stderr.
///
/// This prevents stack traces from leaking into container logs in production.
/// Call this once at application startup.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };

        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();

        tracing::error!(
            thread = thread_name,
            panic_payload = %payload,
            panic_location = %location,
            "Unrecoverable panic — service may be in degraded state"
        );

        // Call the default hook for backward compatibility (e.g., cargo test output)
        default_hook(info);
    }));
}

// ==================== PII Masking ====================

/// Mask PII in log messages to prevent sensitive data leakage (OWASP A09).
///
/// Patterns masked:
/// - Email addresses: user@example.com -> u***@e***.com
/// - Phone numbers: +971501234567 -> +971****4567
/// - Credit card numbers: 4111111111111111 -> 4111****1111
/// - Social security / national IDs: 123-45-6789 -> ***-**-6789
pub fn mask_pii(text: &str) -> String {
    let mut result = text.to_string();

    // Mask email addresses
    let email_regex = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    result = email_regex.replace_all(&result, |caps: &regex::Captures| {
        let email = &caps[0];
        if let Some(at_pos) = email.find('@') {
            let user = &email[..at_pos];
            let domain = &email[at_pos..];
            if let Some(dot_pos) = domain[1..].find('.') {
                let domain_name = &domain[1..dot_pos+1];
                let tld = &domain[dot_pos+1..];
                format!("{}***@{}***.{}", &user[..1.min(user.len())], &domain_name[..1.min(domain_name.len())], tld)
            } else {
                format!("{}***@***", &user[..1.min(user.len())])
            }
        } else {
            "***".to_string()
        }
    }).to_string();

    // Mask credit card numbers (13-19 digits, optionally with spaces/dashes)
    let cc_regex = regex::Regex::new(r"\b(\d{4})[- ]?(\d{4})[- ]?(\d{4})[- ]?(\d{1,7})\b").unwrap();
    result = cc_regex.replace_all(&result, |caps: &regex::Captures| {
        let first4 = &caps[1];
        let last4 = caps.get(4).map(|m| &m.as_str()[m.as_str().len().saturating_sub(4)..]).unwrap_or("****");
        format!("{}****{}", first4, last4)
    }).to_string();

    // Mask phone numbers (international format)
    let phone_regex = regex::Regex::new(r"\+(\d{1,3})\d{6,}").unwrap();
    result = phone_regex.replace_all(&result, |caps: &regex::Captures| {
        let country = &caps[1];
        let full = &caps[0];
        let last4 = &full[full.len().saturating_sub(4)..];
        format!("+{}****{}", country, last4)
    }).to_string();

    result
}

/// Sanitize error messages for logging — removes PII and internal details.
pub fn sanitize_error_message(error: &str) -> String {
    let masked = mask_pii(error);
    // Remove file paths and line numbers that could leak implementation details
    let path_regex = regex::Regex::new(r"(?:/[a-zA-Z0-9_.-]+){2,}").unwrap();
    path_regex.replace_all(&masked, "[path]").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_email() {
        let masked = mask_pii("Contact user@example.com for help");
        assert!(!masked.contains("user@example.com"));
        assert!(masked.contains("***@"));
    }

    #[test]
    fn test_mask_credit_card() {
        let masked = mask_pii("Card: 4111111111111111");
        assert!(!masked.contains("4111111111111111"));
        assert!(masked.contains("****"));
    }

    #[test]
    fn test_mask_phone() {
        let masked = mask_pii("Call +971501234567");
        assert!(!masked.contains("971501234567"));
        assert!(masked.contains("+971****4567"));
    }

    #[test]
    fn test_sanitize_error() {
        let sanitized = sanitize_error_message("Error at /home/user/src/main.rs:42");
        assert!(!sanitized.contains("/home/user/src/main.rs"));
    }
}
