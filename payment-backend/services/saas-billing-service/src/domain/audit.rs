//! Audit Log domain model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An audit log entry for compliance tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub log_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Common audit actions.
pub struct AuditActions;

impl AuditActions {
    // Authentication
    pub const LOGIN: &str = "login";
    pub const LOGOUT: &str = "logout";
    pub const LOGIN_FAILED: &str = "login_failed";
    pub const PASSWORD_CHANGED: &str = "password_changed";
    pub const MFA_ENABLED: &str = "mfa_enabled";
    pub const MFA_DISABLED: &str = "mfa_disabled";

    // Subscription
    pub const SUBSCRIPTION_CREATED: &str = "subscription_created";
    pub const SUBSCRIPTION_UPDATED: &str = "subscription_updated";
    pub const SUBSCRIPTION_CANCELED: &str = "subscription_canceled";
    pub const SUBSCRIPTION_RENEWED: &str = "subscription_renewed";

    // Payment
    pub const PAYMENT_INTENT_CREATED: &str = "payment_intent_created";
    pub const PAYMENT_INTENT_AUTHORIZED: &str = "payment_intent_authorized";
    pub const PAYMENT_INTENT_CAPTURED: &str = "payment_intent_captured";
    pub const PAYMENT_INTENT_REFUNDED: &str = "payment_intent_refunded";
    pub const PAYMENT_INTENT_VOIDED: &str = "payment_intent_voided";

    // Gateway
    pub const GATEWAY_PROFILE_CREATED: &str = "gateway_profile_created";
    pub const GATEWAY_PROFILE_UPDATED: &str = "gateway_profile_updated";
    pub const GATEWAY_PROFILE_DELETED: &str = "gateway_profile_deleted";
    pub const GATEWAY_CREDENTIALS_ROTATED: &str = "gateway_credentials_rotated";

    // Team
    pub const TEAM_MEMBER_INVITED: &str = "team_member_invited";
    pub const TEAM_MEMBER_ACCEPTED: &str = "team_member_accepted";
    pub const TEAM_MEMBER_SUSPENDED: &str = "team_member_suspended";
    pub const TEAM_MEMBER_ROLE_CHANGED: &str = "team_member_role_changed";

    // API Keys
    pub const API_KEY_CREATED: &str = "api_key_created";
    pub const API_KEY_REVOKED: &str = "api_key_revoked";
    pub const API_KEY_ROTATED: &str = "api_key_rotated";

    // Settings
    pub const SETTINGS_UPDATED: &str = "settings_updated";
    pub const WEBHOOK_CREATED: &str = "webhook_created";
    pub const WEBHOOK_UPDATED: &str = "webhook_updated";
    pub const WEBHOOK_DELETED: &str = "webhook_deleted";
}

impl AuditLog {
    /// Create a new audit log entry.
    pub fn new(
        operator_id: Uuid,
        principal_id: Uuid,
        action: &str,
        resource: &str,
        resource_id: Option<String>,
    ) -> Self {
        Self {
            log_id: Uuid::now_v7(),
            operator_id,
            principal_id,
            action: action.into(),
            resource: resource.into(),
            resource_id,
            old_value: None,
            new_value: None,
            ip_address: None,
            user_agent: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Set the old value (before change).
    pub fn with_old_value(mut self, value: serde_json::Value) -> Self {
        self.old_value = Some(value);
        self
    }

    /// Set the new value (after change).
    pub fn with_new_value(mut self, value: serde_json::Value) -> Self {
        self.new_value = Some(value);
        self
    }

    /// Set the IP address.
    pub fn with_ip_address(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Set the user agent.
    pub fn with_user_agent(mut self, ua: String) -> Self {
        self.user_agent = Some(ua);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if this is a sensitive action that requires special logging.
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self.action.as_str(),
            AuditActions::LOGIN
                | AuditActions::LOGOUT
                | AuditActions::LOGIN_FAILED
                | AuditActions::PASSWORD_CHANGED
                | AuditActions::MFA_ENABLED
                | AuditActions::MFA_DISABLED
                | AuditActions::SUBSCRIPTION_CANCELED
                | AuditActions::GATEWAY_CREDENTIALS_ROTATED
                | AuditActions::API_KEY_CREATED
                | AuditActions::API_KEY_REVOKED
        )
    }
}

impl std::fmt::Display for AuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AuditLog({} {} {} {:?})",
            self.action, self.resource, self.resource_id.as_deref().unwrap_or_default(), self.created_at
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_audit_log() -> AuditLog {
        AuditLog::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            AuditActions::LOGIN,
            "principal",
            Some(Uuid::now_v7().to_string()),
        )
    }

    #[test]
    fn test_audit_log_new() {
        let log = test_audit_log();
        assert_eq!(log.action, AuditActions::LOGIN);
        assert_eq!(log.resource, "principal");
        assert!(log.old_value.is_none());
        assert!(log.new_value.is_none());
    }

    #[test]
    fn test_audit_log_with_values() {
        let log = test_audit_log()
            .with_old_value(serde_json::json!({"status": "active"}))
            .with_new_value(serde_json::json!({"status": "suspended"}));
        assert!(log.old_value.is_some());
        assert!(log.new_value.is_some());
    }

    #[test]
    fn test_audit_log_is_sensitive() {
        let mut log = test_audit_log();
        assert!(log.is_sensitive());

        log.action = AuditActions::PAYMENT_INTENT_CREATED.into();
        assert!(!log.is_sensitive());
    }

    #[test]
    fn test_audit_actions() {
        assert_eq!(AuditActions::LOGIN, "login");
        assert_eq!(AuditActions::LOGOUT, "logout");
        assert_eq!(AuditActions::SUBSCRIPTION_CREATED, "subscription_created");
        assert_eq!(AuditActions::GATEWAY_PROFILE_CREATED, "gateway_profile_created");
    }
}
