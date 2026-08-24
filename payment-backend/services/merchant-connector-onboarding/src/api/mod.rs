use crate::commands::*;
use crate::domain::{ConnectorInfo, OnboardingError, OnboardingRequest};
use crate::queries::*;
use uuid::Uuid;

pub struct OnboardingApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl OnboardingApi {
    pub fn new(ch: Box<dyn CommandHandler>, qh: Box<dyn QueryHandler>) -> Self {
        Self { command_handler: ch, query_handler: qh }
    }

    pub async fn initiate_onboarding(&self, cmd: InitiateOnboardingCommand) -> Result<OnboardingRequest, OnboardingError> {
        self.command_handler.initiate_onboarding(cmd).await
    }

    pub async fn submit_credentials(&self, cmd: SubmitCredentialsCommand) -> Result<OnboardingRequest, OnboardingError> {
        self.command_handler.submit_credentials(cmd).await
    }

    pub async fn test_connection(&self, cmd: TestConnectionCommand) -> Result<OnboardingRequest, OnboardingError> {
        self.command_handler.test_connection(cmd).await
    }

    pub async fn complete_test(&self, cmd: CompleteTestCommand) -> Result<OnboardingRequest, OnboardingError> {
        self.command_handler.complete_test(cmd).await
    }

    pub async fn deactivate_link(&self, cmd: DeactivateLinkCommand) -> Result<OnboardingRequest, OnboardingError> {
        self.command_handler.deactivate_link(cmd).await
    }

    pub async fn revoke_link(&self, cmd: RevokeLinkCommand) -> Result<OnboardingRequest, OnboardingError> {
        self.command_handler.revoke_link(cmd).await
    }

    pub async fn get_onboarding(&self, id: Uuid) -> Result<OnboardingRequest, OnboardingError> {
        self.query_handler.get_onboarding(id).await
    }

    pub async fn list_connectors(&self) -> Vec<ConnectorInfo> {
        self.query_handler.list_connectors().await
    }

    pub async fn get_connector_schema(&self, connector_id: &str) -> Result<ConnectorInfo, OnboardingError> {
        self.query_handler.get_connector_schema(connector_id).await
    }

    pub async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        self.query_handler.find_by_operator(operator_id).await
    }
}
