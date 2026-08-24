//! Query handlers for merchant-connector-onboarding

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_onboarding(&self, id: Uuid) -> Result<OnboardingRequest, OnboardingError>;
    async fn list_connectors(&self) -> Vec<ConnectorInfo>;
    async fn get_connector_schema(&self, connector_id: &str) -> Result<ConnectorInfo, OnboardingError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError>;
    async fn find_active(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError>;
}

pub struct OnboardingQueryHandler<R: OnboardingRepository> {
    repo: R,
}

impl<R: OnboardingRepository> OnboardingQueryHandler<R> {
    pub fn new(repo: R) -> Self { Self { repo } }
}

#[async_trait]
impl<R: OnboardingRepository + Send + Sync> QueryHandler for OnboardingQueryHandler<R> {
    async fn get_onboarding(&self, id: Uuid) -> Result<OnboardingRequest, OnboardingError> {
        self.repo.load(id).await?.ok_or(OnboardingError::NotFound(id))
    }

    async fn list_connectors(&self) -> Vec<ConnectorInfo> {
        default_connectors()
    }

    async fn get_connector_schema(&self, connector_id: &str) -> Result<ConnectorInfo, OnboardingError> {
        default_connectors().into_iter().find(|c| c.connector_id == connector_id)
            .ok_or_else(|| OnboardingError::ConnectorNotFound(connector_id.into()))
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        self.repo.find_by_operator(operator_id).await
    }

    async fn find_active(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        self.repo.find_active(operator_id).await
    }
}
