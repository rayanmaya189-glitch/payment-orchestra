//! Command handlers for merchant-connector-onboarding

use async_trait::async_trait;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn initiate_onboarding(&self, cmd: InitiateOnboardingCommand) -> Result<OnboardingRequest, OnboardingError>;
    async fn submit_credentials(&self, cmd: SubmitCredentialsCommand) -> Result<OnboardingRequest, OnboardingError>;
    async fn test_connection(&self, cmd: TestConnectionCommand) -> Result<OnboardingRequest, OnboardingError>;
    async fn complete_test(&self, cmd: CompleteTestCommand) -> Result<OnboardingRequest, OnboardingError>;
    async fn deactivate_link(&self, cmd: DeactivateLinkCommand) -> Result<OnboardingRequest, OnboardingError>;
    async fn revoke_link(&self, cmd: RevokeLinkCommand) -> Result<OnboardingRequest, OnboardingError>;
}

pub struct OnboardingCommandHandler<R: OnboardingRepository> {
    repo: R,
}

impl<R: OnboardingRepository> OnboardingCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: OnboardingRepository + Send + Sync> CommandHandler for OnboardingCommandHandler<R> {
    async fn initiate_onboarding(&self, cmd: InitiateOnboardingCommand) -> Result<OnboardingRequest, OnboardingError> {
        let connectors = default_connectors();
        if !connectors.iter().any(|c| c.connector_id == cmd.connector_id) {
            return Err(OnboardingError::ConnectorNotFound(cmd.connector_id));
        }

        let request = OnboardingRequest::new(
            cmd.operator_id,
            cmd.connector_id,
            cmd.display_name,
            cmd.environment,
        );

        self.repo.save(&request).await?;
        Ok(request)
    }

    async fn submit_credentials(&self, cmd: SubmitCredentialsCommand) -> Result<OnboardingRequest, OnboardingError> {
        let mut request = self.repo.load(cmd.link_id).await?.ok_or(OnboardingError::NotFound(cmd.link_id))?;

        let connectors = default_connectors();
        let schema = connectors.iter().find(|c| c.connector_id == request.connector_id)
            .ok_or(OnboardingError::ConnectorNotFound(request.connector_id.clone()))?;

        request.submit_credentials(cmd.credentials, schema)?;
        self.repo.save(&request).await?;
        Ok(request)
    }

    async fn test_connection(&self, cmd: TestConnectionCommand) -> Result<OnboardingRequest, OnboardingError> {
        let mut request = self.repo.load(cmd.link_id).await?.ok_or(OnboardingError::NotFound(cmd.link_id))?;

        request.start_test()?;

        let mock_result = ConnectionTestResult {
            success: true,
            latency_ms: 245,
            error: None,
            merchant_name: Some("Test Merchant".into()),
            permissions: vec!["authorize".into(), "capture".into(), "refund".into(), "void".into()],
        };

        request.record_test_success(mock_result)?;
        self.repo.save(&request).await?;
        Ok(request)
    }

    async fn complete_test(&self, cmd: CompleteTestCommand) -> Result<OnboardingRequest, OnboardingError> {
        let mut request = self.repo.load(cmd.link_id).await?.ok_or(OnboardingError::NotFound(cmd.link_id))?;

        if request.status == OnboardingStatus::CredentialsSubmitted {
            request.start_test()?;
        }

        if cmd.result.success {
            request.record_test_success(cmd.result)?;
        } else {
            request.record_test_failure(cmd.result)?;
        }

        self.repo.save(&request).await?;
        Ok(request)
    }

    async fn deactivate_link(&self, cmd: DeactivateLinkCommand) -> Result<OnboardingRequest, OnboardingError> {
        let mut request = self.repo.load(cmd.link_id).await?.ok_or(OnboardingError::NotFound(cmd.link_id))?;

        request.deactivate()?;
        self.repo.save(&request).await?;
        Ok(request)
    }

    async fn revoke_link(&self, cmd: RevokeLinkCommand) -> Result<OnboardingRequest, OnboardingError> {
        let mut request = self.repo.load(cmd.link_id).await?.ok_or(OnboardingError::NotFound(cmd.link_id))?;

        request.revoke()?;
        self.repo.save(&request).await?;
        Ok(request)
    }
}
