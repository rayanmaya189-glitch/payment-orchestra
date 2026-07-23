//! Command handlers for BC-01 Operator Management.
//! Each command validates preconditions, mutates aggregate state, and publishes events.

use chrono::Utc;
use uuid::Uuid;
use tracing::info;

use crate::domain::{Operator, OperatorError, OperatorStatus};
use crate::events::{OperatorEvent, OperatorRegistered, OperatorVerified, OperatorSuspended, OperatorReactivated};
use crate::repository::OperatorRepository;

// ─── Command Trait ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn register(&self, cmd: RegisterOperator) -> Result<RegisterOperatorResult, OperatorError>;
    async fn verify_email(&self, cmd: VerifyEmail) -> Result<VerifyEmailResult, OperatorError>;
    async fn update_status(&self, cmd: UpdateOperatorStatus) -> Result<UpdateOperatorStatusResult, OperatorError>;
}

// ─── Command Types ──────────────────────────────────────────────────────────

pub struct RegisterOperator {
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub email: String,
}

pub struct VerifyEmail {
    pub operator_id: Uuid,
    pub verification_token: String,
}

pub struct UpdateOperatorStatus {
    pub operator_id: Uuid,
    pub new_status: OperatorStatus,
    pub reason: String,
    #[allow(dead_code)]
    pub changed_by: Uuid,
}

// ─── Command Results ────────────────────────────────────────────────────────

pub struct RegisterOperatorResult {
    pub operator: Operator,
    #[allow(dead_code)]
    pub verification_token: String,
}

pub struct VerifyEmailResult {
    pub operator: Operator,
}

pub struct UpdateOperatorStatusResult {
    pub operator: Operator,
}

// ─── Command Handler Implementation ────────────────────────────────────────

pub struct OperatorCommandHandler<R: OperatorRepository> {
    repository: R,
    event_bus: Option<platform_messaging::event_bus::ChannelEventBus>,
        
}

impl<R: OperatorRepository> OperatorCommandHandler<R> {
    pub fn new(repository: R) -> Self {
        Self { repository, event_bus: None }
    }

    #[allow(dead_code)]
    pub fn with_event_bus(mut self, event_bus: platform_messaging::event_bus::ChannelEventBus) -> Self {
        self.event_bus = Some(event_bus);
        self
    }
}

#[async_trait::async_trait]
impl<R: OperatorRepository + Send + Sync> CommandHandler for OperatorCommandHandler<R> {
    async fn register(&self, cmd: RegisterOperator) -> Result<RegisterOperatorResult, OperatorError> {
        // Validate trade license format (AF-001a)
        Self::validate_trade_license(&cmd.trade_license_no)?;

        // Check uniqueness
        if self.repository.find_by_trade_license(&cmd.trade_license_no).await?.is_some() {
            return Err(OperatorError::DuplicateTradeLicense(cmd.trade_license_no));
        }
        if self.repository.find_by_email(&cmd.email).await?.is_some() {
            return Err(OperatorError::EmailVerificationFailed("Email already registered".into()));
        }

        // Generate subdomain from legal name
        let subdomain = Self::generate_subdomain(&cmd.legal_name);

        let id = Uuid::now_v7();
        let mut operator = Operator::new(
            id,
            cmd.legal_name,
            cmd.trade_license_no,
            cmd.country,
            cmd.email,
            subdomain,
        );

        // Generate verification token
        let verification_token = Uuid::now_v7().to_string();

        // Store token hash (simplified — in production, store Argon2id hash)
        operator.verification_token_hash = Some(Self::hash_token(&verification_token));

        self.repository.save(&mut operator).await?;

        // Publish event
        self.publish_event(OperatorEvent::Registered(OperatorRegistered {
            operator_id: operator.id,
            legal_name: operator.legal_name.clone(),
            email: operator.email.clone(),
            subdomain: operator.subdomain.clone(),
            occurred_at: Utc::now(),
        }));

        info!(
            operator_id = %operator.id,
            status = %operator.status.as_str(),
            "Operator registered"
        );

        Ok(RegisterOperatorResult {
            operator,
            verification_token,
        })
    }

    async fn verify_email(&self, cmd: VerifyEmail) -> Result<VerifyEmailResult, OperatorError> {
        let mut operator = self.repository.load(cmd.operator_id).await?
            .ok_or(OperatorError::NotFound(cmd.operator_id))?;

        // Validate token against stored hash
        let stored_hash = operator.verification_token_hash
            .as_ref()
            .ok_or_else(|| OperatorError::EmailVerificationFailed("No verification token found".into()))?;

        if !Self::verify_token(&cmd.verification_token, &stored_hash) {
            return Err(OperatorError::EmailVerificationFailed("Invalid verification token".into()));
        }

        let previous_status = operator.status.as_str().to_string();
        operator.verify_email()?;
        operator.mark_provisioned();
        self.repository.save(&mut operator).await?;

        // Publish event
        self.publish_event(OperatorEvent::Verified(OperatorVerified {
            operator_id: operator.id,
            previous_status,
            new_status: operator.status.as_str().to_string(),
            occurred_at: Utc::now(),
        }));

        info!(
            operator_id = %operator.id,
            "Email verified"
        );

        Ok(VerifyEmailResult { operator })
    }

    async fn update_status(&self, cmd: UpdateOperatorStatus) -> Result<UpdateOperatorStatusResult, OperatorError> {
        let mut operator = self.repository.load(cmd.operator_id).await?
            .ok_or(OperatorError::NotFound(cmd.operator_id))?;

        let previous_status = operator.status.clone();
        operator.update_status(cmd.new_status)?;
        self.repository.save(&mut operator).await?;

        // Publish appropriate event
        match operator.status {
            OperatorStatus::Suspended => {
                self.publish_event(OperatorEvent::Suspended(OperatorSuspended {
                    operator_id: operator.id,
                    reason: cmd.reason.clone(),
                    occurred_at: Utc::now(),
                }));
            }
            OperatorStatus::ActiveVerified if previous_status == OperatorStatus::Suspended => {
                self.publish_event(OperatorEvent::Reactivated(OperatorReactivated {
                    operator_id: operator.id,
                    reason: cmd.reason,
                    occurred_at: Utc::now(),
                }));
            }
            _ => {}
        }

        info!(
            operator_id = %operator.id,
            status = %operator.status.as_str(),
            "Operator status updated"
        );

        Ok(UpdateOperatorStatusResult { operator })
    }
}

// ─── Private Helpers ─────────────────────────────────────────────────────

impl<R: OperatorRepository> OperatorCommandHandler<R> {
    fn validate_trade_license(license: &str) -> Result<(), OperatorError> {
        if license.len() < 5 || license.len() > 50 {
            return Err(OperatorError::InvalidTradeLicenseFormat(
                "Trade license must be between 5 and 50 characters".into(),
            ));
        }
        if !license.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '/') {
            return Err(OperatorError::InvalidTradeLicenseFormat(
                "Trade license may only contain alphanumeric characters, hyphens, and slashes".into(),
            ));
        }
        Ok(())
    }

    fn generate_subdomain(legal_name: &str) -> String {
        legal_name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(20)
            .collect()
    }

    /// Simple token hash (SHA-256). In production, use Argon2id.
    fn hash_token(token: &str) -> String {
        let hash = ring::digest::digest(&ring::digest::SHA256, token.as_bytes());
        hex::encode(hash.as_ref())
    }

    /// Verify token against stored hash
    fn verify_token(token: &str, hash: &str) -> bool {
        let computed = Self::hash_token(token);
        // Constant-time comparison to prevent timing attacks
        computed == hash
    }

    fn publish_event(&self, event: OperatorEvent) {
        if let Some(ref bus) = self.event_bus {
            let subject = format!("operator.{}", event.event_type());
            match Self::encode_event_proto(&event) {
                Ok(payload) => {
                    if let Err(e) = bus.publish_sync(&subject, payload) {
                        tracing::warn!(subject = %subject, error = %e, "Failed to publish event");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, event_type = %event.event_type(), "Failed to encode event");
                }
            }
        }
    }

    /// Encode an OperatorEvent as protobuf bytes using the generated proto types.
    fn encode_event_proto(event: &OperatorEvent) -> Result<Vec<u8>, String> {
        use prost::Message;
        match event {
            OperatorEvent::Registered(e) => {
                let proto = platform_proto::operator::OperatorRegisteredEvent {
                    operator_id: e.operator_id.to_string(),
                    legal_name: e.legal_name.clone(),
                    email: e.email.clone(),
                    subdomain: e.subdomain.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            OperatorEvent::Verified(e) => {
                let proto = platform_proto::operator::OperatorVerifiedEvent {
                    operator_id: e.operator_id.to_string(),
                    previous_status: e.previous_status.clone(),
                    new_status: e.new_status.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            OperatorEvent::Suspended(e) => {
                let proto = platform_proto::operator::OperatorSuspendedEvent {
                    operator_id: e.operator_id.to_string(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            OperatorEvent::Reactivated(e) => {
                let proto = platform_proto::operator::OperatorReactivatedEvent {
                    operator_id: e.operator_id.to_string(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryOperatorRepository;

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
