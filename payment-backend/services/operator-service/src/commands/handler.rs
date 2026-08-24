//! Command handlers for BC-01 Operator Management.
//!
//! Extracted per CONVENTIONS.md: one concept per file.

pub use super::types::*;

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::events::OperatorEvent;
use crate::repository::OperatorRepository;

// ─── Command Trait ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn register(&self, cmd: RegisterOperator) -> Result<RegisterOperatorResult, crate::domain::OperatorError>;
    async fn verify_email(&self, cmd: VerifyEmail) -> Result<VerifyEmailResult, crate::domain::OperatorError>;
    async fn update_status(&self, cmd: UpdateOperatorStatus) -> Result<UpdateOperatorStatusResult, crate::domain::OperatorError>;
}

// ─── Command Handler Implementation ────────────────────────────────────────

pub struct OperatorCommandHandler<R: OperatorRepository> {
    pub(crate) repository: R,
    pub(crate) event_bus: Option<Arc<dyn EventBus>>,
}

impl<R: OperatorRepository> OperatorCommandHandler<R> {
    pub fn new(repository: R) -> Self {
        Self { repository, event_bus: None }
    }

    pub fn with_event_bus(mut self, event_bus: Arc<dyn EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub(crate) fn publish_event(&self, event: OperatorEvent) {
        match Self::encode_event_proto(&event) {
            Ok(payload) => {
                publish_event_fire_and_forget(&self.event_bus, "operator", event.event_type(), payload);
            }
            Err(e) => {
                tracing::error!(error = %e, event_type = %event.event_type(), "Failed to encode event");
            }
        }
    }

    /// Encode an OperatorEvent as protobuf bytes using the generated proto types.
    fn encode_event_proto(event: &OperatorEvent) -> Result<Vec<u8>, String> {
        match event {
            OperatorEvent::Registered(e) => {
                let proto = platform_proto::operator::OperatorRegisteredEvent {
                    operator_id: e.operator_id.to_string(),
                    legal_name: e.legal_name.clone(),
                    email: e.email.clone(),
                    subdomain: e.subdomain.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            OperatorEvent::Verified(e) => {
                let proto = platform_proto::operator::OperatorVerifiedEvent {
                    operator_id: e.operator_id.to_string(),
                    previous_status: e.previous_status.clone(),
                    new_status: e.new_status.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            OperatorEvent::Suspended(e) => {
                let proto = platform_proto::operator::OperatorSuspendedEvent {
                    operator_id: e.operator_id.to_string(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            OperatorEvent::Reactivated(e) => {
                let proto = platform_proto::operator::OperatorReactivatedEvent {
                    operator_id: e.operator_id.to_string(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
        }
    }

    pub(crate) fn validate_trade_license(license: &str) -> Result<(), crate::domain::OperatorError> {
        if license.len() < 5 || license.len() > 50 {
            return Err(crate::domain::OperatorError::InvalidTradeLicenseFormat(
                "Trade license must be between 5 and 50 characters".into(),
            ));
        }
        if !license.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '/') {
            return Err(crate::domain::OperatorError::InvalidTradeLicenseFormat(
                "Trade license may only contain alphanumeric characters, hyphens, and slashes".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn generate_subdomain(legal_name: &str) -> String {
        legal_name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(20)
            .collect()
    }

    /// Simple token hash (SHA-256). In production, use Argon2id.
    pub(crate) fn hash_token(token: &str) -> String {
        let hash = ring::digest::digest(&ring::digest::SHA256, token.as_bytes());
        hex::encode(hash.as_ref())
    }

    /// Verify token against stored hash
    pub(crate) fn verify_token(token: &str, hash: &str) -> bool {
        let computed = Self::hash_token(token);
        computed == hash
    }
}

#[async_trait::async_trait]
impl<R: OperatorRepository + Send + Sync> CommandHandler for OperatorCommandHandler<R> {
    async fn register(&self, cmd: RegisterOperator) -> Result<RegisterOperatorResult, crate::domain::OperatorError> {
        self.register_impl(cmd).await
    }

    async fn verify_email(&self, cmd: VerifyEmail) -> Result<VerifyEmailResult, crate::domain::OperatorError> {
        self.verify_email_impl(cmd).await
    }

    async fn update_status(&self, cmd: UpdateOperatorStatus) -> Result<UpdateOperatorStatusResult, crate::domain::OperatorError> {
        self.update_status_impl(cmd).await
    }
}

// ─── Blanket impl: Box<dyn CommandHandler> delegates to inner ────────────────

#[async_trait::async_trait]
impl<T: CommandHandler + ?Sized> CommandHandler for Box<T> {
    async fn register(&self, cmd: RegisterOperator) -> Result<RegisterOperatorResult, crate::domain::OperatorError> {
        (**self).register(cmd).await
    }

    async fn verify_email(&self, cmd: VerifyEmail) -> Result<VerifyEmailResult, crate::domain::OperatorError> {
        (**self).verify_email(cmd).await
    }

    async fn update_status(&self, cmd: UpdateOperatorStatus) -> Result<UpdateOperatorStatusResult, crate::domain::OperatorError> {
        (**self).update_status(cmd).await
    }
}

