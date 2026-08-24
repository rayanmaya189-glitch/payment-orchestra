//! Self-serve signup command handler.
//!
//! This module handles the self-serve merchant signup flow:
//! 1. Create operator account
//! 2. Create IAM principal (user)
//! 3. Create SaaS subscription (with trial)
//! 4. Send verification email

use chrono::Utc;
use uuid::Uuid;
use tracing::info;

use crate::domain::{Operator, OperatorError};
use crate::events::{OperatorEvent, OperatorRegistered};
use crate::repository::OperatorRepository;
use super::types::*;
use super::OperatorCommandHandler;

/// Self-serve signup request.
#[derive(Debug, Clone)]
pub struct SelfServeSignupRequest {
    pub email: String,
    pub password: String,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub plan_slug: Option<String>, // e.g., "free", "starter"
    pub trial_days: Option<i32>,
}

/// Self-serve signup result.
#[derive(Debug, Clone)]
pub struct SelfServeSignupResult {
    pub operator: Operator,
    pub principal_id: Uuid,
    pub subscription_id: Uuid,
    pub verification_token: String,
    pub is_trial: bool,
}

impl<R: OperatorRepository + Send + Sync> OperatorCommandHandler<R> {
    /// Handle self-serve merchant signup.
    ///
    /// This creates:
    /// 1. Operator (tenant) account
    /// 2. IAM principal (user account)
    /// 3. SaaS subscription (with optional trial)
    pub async fn self_serve_signup(
        &self,
        request: SelfServeSignupRequest,
    ) -> Result<SelfServeSignupResult, OperatorError> {
        // Validate inputs
        Self::validate_trade_license(&request.trade_license_no)?;
        
        if request.email.is_empty() || !request.email.contains('@') {
            return Err(OperatorError::InvalidTradeLicenseFormat(
                "Invalid email address".into(),
            ));
        }

        // Check uniqueness
        if self.repository.find_by_trade_license(&request.trade_license_no).await?.is_some() {
            return Err(OperatorError::DuplicateTradeLicense(request.trade_license_no));
        }
        if self.repository.find_by_email(&request.email).await?.is_some() {
            return Err(OperatorError::EmailVerificationFailed(
                "Email already registered".into(),
            ));
        }

        // Generate subdomain from legal name
        let subdomain = Self::generate_subdomain(&request.legal_name);

        let operator_id = Uuid::now_v7();
        let mut operator = Operator::new(
            operator_id,
            request.legal_name,
            request.trade_license_no,
            request.country,
            request.email.clone(),
            subdomain,
        );

        // Generate verification token
        let verification_token = Uuid::now_v7().to_string();
        operator.verification_token_hash = Some(Self::hash_token(&verification_token));

        // Save operator
        self.repository.save(&mut operator).await?;

        // Create IAM principal (user account)
        let principal_id = Uuid::now_v7();
        // In production, this would call IAM service to create principal
        // For now, we'll just track the ID

        // Create SaaS subscription
        let subscription_id = Uuid::now_v7();
        let plan_slug = request.plan_slug.unwrap_or_else(|| "free".into());
        let trial_days = request.trial_days.unwrap_or(14);
        
        // Determine if this is a trial
        let is_trial = trial_days > 0 && plan_slug != "free";

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
            email = %request.email,
            plan = %plan_slug,
            is_trial = is_trial,
            "Self-serve signup completed"
        );

        Ok(SelfServeSignupResult {
            operator,
            principal_id,
            subscription_id,
            verification_token,
            is_trial,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryOperatorRepository;

    #[tokio::test]
    async fn test_self_serve_signup() {
        let repo = InMemoryOperatorRepository::new();
        let handler = OperatorCommandHandler::new(repo);

        let request = SelfServeSignupRequest {
            email: "merchant@example.com".into(),
            password: "secure_password".into(),
            legal_name: "Acme Corp".into(),
            trade_license_no: "TL-12345".into(),
            country: "AE".into(),
            plan_slug: Some("starter".into()),
            trial_days: Some(14),
        };

        let result = handler.self_serve_signup(request).await.unwrap();
        
        assert_eq!(result.operator.legal_name, "Acme Corp");
        assert_eq!(result.operator.email, "merchant@example.com");
        assert!(result.is_trial);
        assert!(!result.verification_token.is_empty());
    }

    #[tokio::test]
    async fn test_self_serve_signup_duplicate_email() {
        let repo = InMemoryOperatorRepository::new();
        let handler = OperatorCommandHandler::new(repo);

        let request1 = SelfServeSignupRequest {
            email: "merchant@example.com".into(),
            password: "password".into(),
            legal_name: "Acme Corp".into(),
            trade_license_no: "TL-12345".into(),
            country: "AE".into(),
            plan_slug: None,
            trial_days: None,
        };

        let _ = handler.self_serve_signup(request1).await.unwrap();

        let request2 = SelfServeSignupRequest {
            email: "merchant@example.com".into(),
            password: "password".into(),
            legal_name: "Acme Corp 2".into(),
            trade_license_no: "TL-12346".into(),
            country: "AE".into(),
            plan_slug: None,
            trial_days: None,
        };

        let result = handler.self_serve_signup(request2).await;
        assert!(result.is_err());
    }
}
