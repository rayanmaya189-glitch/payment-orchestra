use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::Notification;
use crate::domain::value_objects::NotificationStatus;
use platform_error::PlatformError;

// ==================== Repository ====================

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    /// Find a notification by ID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, PlatformError>;

    /// Upsert a notification (insert or update).
    async fn save(&self, notification: &Notification) -> Result<(), PlatformError>;

    /// Find all notifications eligible for retry (status=Failed, retry_count < max_retries).
    async fn find_retryable(&self) -> Result<Vec<Notification>, PlatformError>;

    /// Find notifications by operator_id, ordered by created_at descending.
    async fn find_by_operator(
        &self,
        operator_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Notification>, PlatformError>;

    /// Count notifications by operator_id.
    async fn count_by_operator(&self, operator_id: Uuid) -> Result<i64, PlatformError>;

    /// Find notifications by status.
    async fn find_by_status(
        &self,
        status: NotificationStatus,
        limit: u64,
    ) -> Result<Vec<Notification>, PlatformError>;
}

// ==================== Providers ====================

/// Email provider interface. Implementations handle actual SMTP/API dispatch.
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send an email and return the provider's message ID.
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<ProviderResult, ProviderError>;
}

/// SMS provider interface.
#[async_trait]
pub trait SmsProvider: Send + Sync {
    /// Send an SMS and return the provider's message ID.
    async fn send_sms(
        &self,
        to: &str,
        body: &str,
    ) -> Result<ProviderResult, ProviderError>;
}

/// Webhook provider interface.
#[async_trait]
pub trait WebhookProvider: Send + Sync {
    /// POST a JSON payload to the webhook URL.
    async fn send_webhook(
        &self,
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<ProviderResult, ProviderError>;
}

// ==================== Provider Result Types ====================

/// Successful provider dispatch result.
#[derive(Debug, Clone)]
pub struct ProviderResult {
    /// Provider-assigned message ID for tracking.
    pub message_id: String,
    /// Provider-specific metadata (e.g., cost, delivery estimate).
    pub metadata: Option<serde_json::Value>,
}

/// Provider-level errors that may be transient or permanent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// Transient failure — eligible for retry (e.g., network timeout, rate limit).
    Transient(String),
    /// Permanent failure — no retry useful (e.g., invalid address, bounced).
    Permanent(String),
    /// Provider is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transient(msg) => write!(f, "Transient provider error: {msg}"),
            Self::Permanent(msg) => write!(f, "Permanent provider error: {msg}"),
            Self::Unavailable(msg) => write!(f, "Provider unavailable: {msg}"),
        }
    }
}

impl ProviderError {
    /// Whether this error is transient and the operation should be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient(_) | Self::Unavailable(_))
    }
}

impl From<ProviderError> for PlatformError {
    fn from(e: ProviderError) -> Self {
        match e {
            ProviderError::Transient(msg) | ProviderError::Unavailable(msg) => {
                PlatformError::Unavailable(msg)
            }
            ProviderError::Permanent(msg) => PlatformError::Validation(
                platform_error::ValidationError::MissingField(msg),
            ),
        }
    }
}

// ==================== Template Store ====================

/// Template storage interface for loading notification templates.
#[async_trait]
pub trait TemplateStore: Send + Sync {
    /// Load a template by ID. Returns (subject_template, body_template).
    async fn load_template(
        &self,
        template_id: &str,
    ) -> Result<Option<(Option<String>, String)>, PlatformError>;
}
