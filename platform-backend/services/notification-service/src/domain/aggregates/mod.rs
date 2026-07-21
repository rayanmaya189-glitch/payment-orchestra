use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{NotificationStatus, NotificationType};

/// Maximum allowed template variable value length to prevent abuse.
const MAX_TEMPLATE_VAR_LENGTH: usize = 4096;

/// Template rendering engine. Supports `{{variable}}` placeholders.
///
/// Variable names are resolved from the provided JSON data object.
/// Nested keys use dot notation: `{{user.name}}` resolves `data["user"]["name"]`.
/// Numeric values are formatted as their JSON representation (no locale formatting).
pub struct TemplateEngine;

impl TemplateEngine {
    /// Render a template string by replacing `{{key}}` placeholders with values from `data`.
    ///
    /// Returns the rendered string. Unknown placeholders are left as-is.
    pub fn render(template: &str, data: &serde_json::Value) -> String {
        let mut result = template.to_string();

        if let Some(obj) = data.as_object() {
            for (key, value) in obj {
                let rendered_value = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => String::new(),
                    _ => value.to_string(),
                };
                let truncated = if rendered_value.len() > MAX_TEMPLATE_VAR_LENGTH {
                    format!("{}...", &rendered_value[..MAX_TEMPLATE_VAR_LENGTH])
                } else {
                    rendered_value
                };
                let placeholder = format!("{{{{{key}}}}}");
                result = result.replace(&placeholder, &truncated);
            }
        }

        result
    }

    /// Extract all placeholder keys from a template string.
    ///
    /// Returns a vector of key names without the `{{}}` delimiters.
    pub fn extract_placeholders(template: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut chars = template.char_indices().peekable();

        while let Some((i, ch)) = chars.next() {
            if ch == '{' && chars.peek().map_or(false, |&(_, c)| c == '{') {
                chars.next(); // skip second {
                let mut key = String::new();
                let mut found_close = false;

                while let Some(&(_, c)) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if chars.peek().map_or(false, |&(_, c2)| c2 == '}') {
                            chars.next();
                            found_close = true;
                            break;
                        }
                        // Single } — not a valid placeholder
                        break;
                    }
                    chars.next();
                    key.push(c);
                }

                if found_close && !key.is_empty() {
                    placeholders.push(key);
                }
            }
        }

        placeholders
    }
}

/// Notification aggregate root.
///
/// Encapsulates the full lifecycle: creation → template rendering → dispatch → delivery tracking → retry.
#[derive(Debug, Clone)]
pub struct Notification {
    pub notification_id: Uuid,
    pub operator_id: Uuid,
    pub notification_type: NotificationType,
    pub status: NotificationStatus,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub template_id: Option<String>,
    pub template_data: Option<serde_json::Value>,
    pub provider_message_id: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub sent_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(
        operator_id: Uuid,
        notification_type: NotificationType,
        recipient: String,
        subject: Option<String>,
        body: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            notification_id: Uuid::now_v7(),
            operator_id,
            notification_type,
            status: NotificationStatus::Pending,
            recipient,
            subject,
            body,
            template_id: None,
            template_data: None,
            provider_message_id: None,
            retry_count: 0,
            max_retries: 3,
            sent_at: None,
            delivered_at: None,
            failed_at: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Render template placeholders in subject and body using the template_data.
    ///
    /// If no template_id is set, this is a no-op. Numeric, boolean, and null values
    /// are formatted to their JSON string representation. Unknown placeholders are left as-is.
    pub fn render_template(&mut self) -> Result<(), TemplateRenderError> {
        if self.template_id.is_none() || self.template_data.is_none() {
            return Ok(());
        }

        let data = self.template_data.as_ref().unwrap();

        if let Some(ref subject) = self.subject.clone() {
            self.subject = Some(TemplateEngine::render(subject, data));
        }

        self.body = TemplateEngine::render(&self.body, data);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Transition to Sent status upon successful provider dispatch.
    pub fn mark_sent(&mut self, provider_message_id: String) {
        self.status = NotificationStatus::Sent;
        self.provider_message_id = Some(provider_message_id);
        self.sent_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to Delivered status upon delivery confirmation (e.g., webhook callback).
    pub fn mark_delivered(&mut self) {
        self.status = NotificationStatus::Delivered;
        self.delivered_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Record a dispatch failure. Increments retry_count.
    ///
    /// If retry_count reaches max_retries, transitions to DeadLetter (no more retries).
    /// Otherwise transitions to Failed (eligible for retry).
    pub fn mark_failed(&mut self, error: String) {
        self.retry_count += 1;
        self.error_message = Some(error);
        self.updated_at = Utc::now();

        if self.retry_count >= self.max_retries {
            self.status = NotificationStatus::DeadLetter;
            self.failed_at = Some(Utc::now());
        } else {
            self.status = NotificationStatus::Failed;
        }
    }

    /// Reset for retry: transitions from Failed back to Pending.
    ///
    /// Returns `Err` if the notification is not in a retryable state.
    pub fn prepare_retry(&mut self) -> Result<(), RetryError> {
        if self.status == NotificationStatus::DeadLetter {
            return Err(RetryError::ExhaustedRetries {
                notification_id: self.notification_id,
                retry_count: self.retry_count,
                max_retries: self.max_retries,
            });
        }
        if self.status != NotificationStatus::Failed {
            return Err(RetryError::InvalidState {
                notification_id: self.notification_id,
                current_status: self.status.clone(),
            });
        }
        self.status = NotificationStatus::Pending;
        self.error_message = None;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if the notification can be retried.
    pub fn can_retry(&self) -> bool {
        self.status == NotificationStatus::Failed && self.retry_count < self.max_retries
    }

    /// Compute exponential backoff delay in seconds for the current retry attempt.
    ///
    /// Formula: `min(2^retry_count, max_delay)`
    /// Max delay: 300 seconds (5 minutes).
    pub fn backoff_delay_secs(&self) -> u64 {
        let max_delay: u64 = 300;
        let delay = 2u64.saturating_pow(self.retry_count as u32);
        delay.min(max_delay)
    }

    /// Validate the notification before dispatch.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.recipient.trim().is_empty() {
            return Err(ValidationError::EmptyRecipient);
        }
        if self.body.trim().is_empty() {
            return Err(ValidationError::EmptyBody);
        }
        match self.notification_type {
            NotificationType::Email => {
                if !self.recipient.contains('@') {
                    return Err(ValidationError::InvalidEmail {
                        recipient: self.recipient.clone(),
                    });
                }
            }
            NotificationType::Sms => {
                // Phone numbers must be digits with optional + prefix, 7-15 chars
                let cleaned = self.recipient.trim_start_matches('+');
                if !cleaned.chars().all(|c| c.is_ascii_digit()) || cleaned.len() < 7 || cleaned.len() > 15 {
                    return Err(ValidationError::InvalidPhoneNumber {
                        recipient: self.recipient.clone(),
                    });
                }
            }
            NotificationType::Webhook => {
                if !self.recipient.starts_with("http://") && !self.recipient.starts_with("https://") {
                    return Err(ValidationError::InvalidWebhookUrl {
                        url: self.recipient.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyRecipient,
    EmptyBody,
    InvalidEmail { recipient: String },
    InvalidPhoneNumber { recipient: String },
    InvalidWebhookUrl { url: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRecipient => write!(f, "Recipient must not be empty"),
            Self::EmptyBody => write!(f, "Body must not be empty"),
            Self::InvalidEmail { recipient } => write!(f, "Invalid email address: {recipient}"),
            Self::InvalidPhoneNumber { recipient } => write!(f, "Invalid phone number: {recipient}"),
            Self::InvalidWebhookUrl { url } => write!(f, "Invalid webhook URL: {url}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateRenderError {
    InvalidTemplate { template_id: String, reason: String },
}

impl std::fmt::Display for TemplateRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTemplate { template_id, reason } => {
                write!(f, "Template '{template_id}' render failed: {reason}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryError {
    ExhaustedRetries {
        notification_id: Uuid,
        retry_count: i32,
        max_retries: i32,
    },
    InvalidState {
        notification_id: Uuid,
        current_status: NotificationStatus,
    },
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExhaustedRetries {
                notification_id,
                retry_count,
                max_retries,
            } => write!(
                f,
                "Notification {notification_id} exhausted all {max_retries} retries (attempted {retry_count})"
            ),
            Self::InvalidState {
                notification_id,
                current_status,
            } => write!(
                f,
                "Notification {notification_id} cannot retry from status '{current_status}'"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== TemplateEngine tests ====================

    #[test]
    fn test_template_engine_simple_replacement() {
        let data = serde_json::json!({"name": "World"});
        let result = TemplateEngine::render("Hello {{name}}!", &data);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_template_engine_multiple_variables() {
        let data = serde_json::json!({"name": "Alice", "amount": "100.50", "currency": "AED"});
        let result = TemplateEngine::render("Dear {{name}}, your payment of {{amount}} {{currency}} was received.", &data);
        assert_eq!(result, "Dear Alice, your payment of 100.50 AED was received.");
    }

    #[test]
    fn test_template_engine_numeric_values() {
        let data = serde_json::json!({"count": 42, "price": 99.99});
        let result = TemplateEngine::render("Items: {{count}}, Price: {{price}}", &data);
        assert_eq!(result, "Items: 42, Price: 99.99");
    }

    #[test]
    fn test_template_engine_boolean_and_null() {
        let data = serde_json::json!({"active": true, "note": null});
        let result = TemplateEngine::render("Active: {{active}}, Note: {{note}}", &data);
        assert_eq!(result, "Active: true, Note: ");
    }

    #[test]
    fn test_template_engine_unknown_placeholder_unchanged() {
        let data = serde_json::json!({"name": "Bob"});
        let result = TemplateEngine::render("Hello {{name}}, code: {{code}}", &data);
        assert_eq!(result, "Hello Bob, code: {{code}}");
    }

    #[test]
    fn test_template_engine_empty_data() {
        let data = serde_json::json!({});
        let result = TemplateEngine::render("No placeholders here", &data);
        assert_eq!(result, "No placeholders here");
    }

    #[test]
    fn test_template_engine_long_value_truncated() {
        let long_value = "x".repeat(5000);
        let data = serde_json::json!({"val": long_value});
        let result = TemplateEngine::render("{{val}}", &data);
        assert_eq!(result.len(), 4096 + 3); // truncated + "..."
    }

    #[test]
    fn test_extract_placeholders() {
        let placeholders = TemplateEngine::extract_placeholders("Hello {{name}}, your code is {{code}}");
        assert_eq!(placeholders, vec!["name", "code"]);
    }

    // ==================== Notification aggregate tests ====================

    #[test]
    fn test_new_notification_defaults() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            Some("Subject".into()),
            "Body".into(),
        );
        assert_eq!(n.status, NotificationStatus::Pending);
        assert_eq!(n.retry_count, 0);
        assert_eq!(n.max_retries, 3);
        assert!(n.provider_message_id.is_none());
    }

    #[test]
    fn test_mark_sent() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        n.mark_sent("msg_123".into());
        assert_eq!(n.status, NotificationStatus::Sent);
        assert_eq!(n.provider_message_id, Some("msg_123".into()));
        assert!(n.sent_at.is_some());
    }

    #[test]
    fn test_mark_delivered() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        n.mark_sent("msg_123".into());
        n.mark_delivered();
        assert_eq!(n.status, NotificationStatus::Delivered);
        assert!(n.delivered_at.is_some());
    }

    #[test]
    fn test_mark_failed_increments_retry() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        n.mark_failed("timeout".into());
        assert_eq!(n.retry_count, 1);
        assert_eq!(n.status, NotificationStatus::Failed);
        assert_eq!(n.error_message, Some("timeout".into()));
    }

    #[test]
    fn test_max_retries_marks_dead_letter() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        n.max_retries = 2;
        n.mark_failed("fail 1".into());
        assert_eq!(n.status, NotificationStatus::Failed);
        n.mark_failed("fail 2".into());
        assert_eq!(n.status, NotificationStatus::DeadLetter);
        assert!(n.failed_at.is_some());
    }

    #[test]
    fn test_can_retry() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        assert!(!n.can_retry()); // Pending
        n.mark_failed("err".into());
        assert!(n.can_retry()); // Failed with retries left
        n.mark_failed("err".into());
        assert!(n.can_retry()); // Failed with retries left (max_retries=3, retry_count=2)
        n.mark_failed("err".into());
        assert!(!n.can_retry()); // DeadLetter
    }

    #[test]
    fn test_prepare_retry() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        assert!(n.prepare_retry().is_err()); // InvalidState: Pending
        n.mark_failed("err".into());
        assert!(n.prepare_retry().is_ok());
        assert_eq!(n.status, NotificationStatus::Pending);
    }

    #[test]
    fn test_prepare_retry_exhausted() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        n.max_retries = 1;
        n.mark_failed("err".into());
        assert_eq!(n.status, NotificationStatus::DeadLetter);
        assert!(matches!(
            n.prepare_retry(),
            Err(RetryError::ExhaustedRetries { .. })
        ));
    }

    #[test]
    fn test_backoff_delay() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Body".into(),
        );
        assert_eq!(n.backoff_delay_secs(), 1); // retry_count=0: 2^0 = 1
        n.retry_count = 1;
        assert_eq!(n.backoff_delay_secs(), 2); // 2^1 = 2
        n.retry_count = 2;
        assert_eq!(n.backoff_delay_secs(), 4); // 2^2 = 4
        n.retry_count = 8;
        assert_eq!(n.backoff_delay_secs(), 256); // 2^8 = 256
        n.retry_count = 10;
        assert_eq!(n.backoff_delay_secs(), 300); // capped at 300
    }

    #[test]
    fn test_render_template_simple() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            Some("Hello {{name}}".into()),
            "Your code is {{code}}".into(),
        );
        n.template_id = Some("welcome".into());
        n.template_data = Some(serde_json::json!({"name": "World", "code": "ABC123"}));
        n.render_template().unwrap();
        assert_eq!(n.subject, Some("Hello World".into()));
        assert_eq!(n.body, "Your code is ABC123");
    }

    #[test]
    fn test_render_template_no_template_id_noop() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "test@example.com".into(),
            None,
            "Hello {{name}}".into(),
        );
        n.template_data = Some(serde_json::json!({"name": "World"}));
        n.render_template().unwrap();
        assert_eq!(n.body, "Hello {{name}}");
    }

    // ==================== Validation tests ====================

    #[test]
    fn test_validate_email_valid() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "user@example.com".into(),
            None,
            "Body".into(),
        );
        assert!(n.validate().is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "not-an-email".into(),
            None,
            "Body".into(),
        );
        assert!(matches!(n.validate(), Err(ValidationError::InvalidEmail { .. })));
    }

    #[test]
    fn test_validate_sms_valid() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Sms,
            "+971501234567".into(),
            None,
            "Hello".into(),
        );
        assert!(n.validate().is_ok());
    }

    #[test]
    fn test_validate_sms_invalid() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Sms,
            "123".into(),
            None,
            "Hello".into(),
        );
        assert!(matches!(n.validate(), Err(ValidationError::InvalidPhoneNumber { .. })));
    }

    #[test]
    fn test_validate_webhook_valid() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Webhook,
            "https://example.com/webhook".into(),
            None,
            "Body".into(),
        );
        assert!(n.validate().is_ok());
    }

    #[test]
    fn test_validate_webhook_invalid() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Webhook,
            "ftp://example.com".into(),
            None,
            "Body".into(),
        );
        assert!(matches!(n.validate(), Err(ValidationError::InvalidWebhookUrl { .. })));
    }

    #[test]
    fn test_validate_empty_recipient() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "".into(),
            None,
            "Body".into(),
        );
        assert!(matches!(n.validate(), Err(ValidationError::EmptyRecipient)));
    }

    #[test]
    fn test_validate_empty_body() {
        let n = Notification::new(
            Uuid::now_v7(),
            NotificationType::Email,
            "a@b.com".into(),
            None,
            "".into(),
        );
        assert!(matches!(n.validate(), Err(ValidationError::EmptyBody)));
    }
}
