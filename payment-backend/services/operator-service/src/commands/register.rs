//! Register operator and verify email command handlers.

use chrono::Utc;
use uuid::Uuid;
use tracing::info;

use crate::domain::{Operator, OperatorError};
use crate::events::{OperatorEvent, OperatorRegistered, OperatorVerified};
use crate::repository::OperatorRepository;
use super::types::*;
use super::OperatorCommandHandler;

impl<R: OperatorRepository + Send + Sync> OperatorCommandHandler<R> {
    pub(crate) async fn register_impl(&self, cmd: RegisterOperator) -> Result<RegisterOperatorResult, OperatorError> {
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

    pub(crate) async fn verify_email_impl(&self, cmd: VerifyEmail) -> Result<VerifyEmailResult, OperatorError> {
        let mut operator = self.repository.load(cmd.operator_id).await?
            .ok_or(OperatorError::NotFound(cmd.operator_id))?;

        // Validate token against stored hash
        let stored_hash = operator.verification_token_hash
            .as_ref()
            .ok_or_else(|| OperatorError::EmailVerificationFailed("No verification token found".into()))?;

        if !Self::verify_token(&cmd.verification_token, stored_hash) {
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
}
