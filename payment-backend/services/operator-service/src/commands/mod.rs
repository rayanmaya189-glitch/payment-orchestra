//! Command handlers for BC-01 Operator Management.

pub mod types;
pub(crate) mod register;
pub(crate) mod status;

pub use types::*;


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

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::repository::InMemoryOperatorRepository;
    use crate::domain::OperatorStatus;

    fn create_handler() -> OperatorCommandHandler<InMemoryOperatorRepository> {
        OperatorCommandHandler::new(InMemoryOperatorRepository::new())
    }

    #[tokio::test]
    async fn test_register_operator_success() {
        let handler = create_handler();
        let result = handler.register(RegisterOperator {
            legal_name: "Acme Corp".into(),
            trade_license_no: "CN-12345".into(),
            country: "AE".into(),
            email: "admin@acme.com".into(),
        }).await.unwrap();

        assert_eq!(result.operator.status, OperatorStatus::Pending);
        assert!(!result.verification_token.is_empty());
        assert!(result.operator.verification_token_hash.is_some());
    }

    #[tokio::test]
    async fn test_register_duplicate_trade_license_rejected() {
        let handler = create_handler();
        handler.register(RegisterOperator {
            legal_name: "Acme Corp".into(),
            trade_license_no: "CN-12345".into(),
            country: "AE".into(),
            email: "admin@acme.com".into(),
        }).await.unwrap();

        let result = handler.register(RegisterOperator {
            legal_name: "Acme Corp 2".into(),
            trade_license_no: "CN-12345".into(),
            country: "AE".into(),
            email: "admin2@acme.com".into(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_email_verification_transitions() {
        let handler = create_handler();
        let registered = handler.register(RegisterOperator {
            legal_name: "Acme Corp".into(),
            trade_license_no: "CN-12345".into(),
            country: "AE".into(),
            email: "admin@acme.com".into(),
        }).await.unwrap();

        let result = handler.verify_email(VerifyEmail {
            operator_id: registered.operator.id,
            verification_token: registered.verification_token,
        }).await.unwrap();

        assert_eq!(result.operator.status, OperatorStatus::ActiveUnverified);
    }

    #[tokio::test]
    async fn test_email_verification_wrong_token_rejected() {
        let handler = create_handler();
        let registered = handler.register(RegisterOperator {
            legal_name: "Acme Corp".into(),
            trade_license_no: "CN-12345".into(),
            country: "AE".into(),
            email: "admin@acme.com".into(),
        }).await.unwrap();

        let result = handler.verify_email(VerifyEmail {
            operator_id: registered.operator.id,
            verification_token: "wrong-token".into(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_nonexistent_operator_fails() {
        let handler = create_handler();
        let result = handler.verify_email(VerifyEmail {
            operator_id: Uuid::now_v7(),
            verification_token: "token".into(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_status_verified() {
        let handler = create_handler();
        let registered = handler.register(RegisterOperator {
            legal_name: "Acme Corp".into(),
            trade_license_no: "CN-12345".into(),
            country: "AE".into(),
            email: "admin@acme.com".into(),
        }).await.unwrap();

        // Verify first
        handler.verify_email(VerifyEmail {
            operator_id: registered.operator.id,
            verification_token: registered.verification_token,
        }).await.unwrap();

        // Update to verified
        let result = handler.update_status(UpdateOperatorStatus {
            operator_id: registered.operator.id,
            new_status: OperatorStatus::ActiveVerified,
            reason: "KYB approved".into(),
            changed_by: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.operator.status, OperatorStatus::ActiveVerified);
    }

    #[tokio::test]
    async fn test_invalid_status_transition_rejected() {
        let handler = create_handler();
        let registered = handler.register(RegisterOperator {
            legal_name: "Acme Corp".into(),
            trade_license_no: "CN-12345".into(),
            country: "AE".into(),
            email: "admin@acme.com".into(),
        }).await.unwrap();

        let result = handler.update_status(UpdateOperatorStatus {
            operator_id: registered.operator.id,
            new_status: OperatorStatus::ActiveVerified,
            reason: "Should not work".into(),
            changed_by: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_trade_license_format() {
        let handler = create_handler();
        let result = handler.register(RegisterOperator {
            legal_name: "Test".into(),
            trade_license_no: "AB".into(),
            country: "AE".into(),
            email: "test@test.com".into(),
        }).await;

        assert!(result.is_err());
    }
}
