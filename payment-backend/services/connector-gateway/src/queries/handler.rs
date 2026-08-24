//! Query handlers for connector-gateway.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{AcquirerConnector, ConnectorRegistry, GatewayProfile, OnboardingSchema};
use crate::repository::GatewayProfileRepository;
use crate::queries::types::*;

// Blanket impl: Box<dyn QueryHandler> implements QueryHandler
#[async_trait]
impl QueryHandler for Box<dyn QueryHandler> {
    async fn get_gateway_profile(&self, id: Uuid) -> Result<Option<GatewayProfile>, String> {
        self.as_ref().get_gateway_profile(id).await
    }
    async fn list_gateway_profiles(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, String> {
        self.as_ref().list_gateway_profiles(operator_id).await
    }
    async fn get_connector_schema(&self, connector_id: &str) -> Result<Option<OnboardingSchema>, String> {
        self.as_ref().get_connector_schema(connector_id).await
    }
    async fn list_connectors(&self) -> Result<Vec<ConnectorInfo>, String> {
        self.as_ref().list_connectors().await
    }
}

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_gateway_profile(&self, id: Uuid) -> Result<Option<GatewayProfile>, String>;
    async fn list_gateway_profiles(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, String>;
    async fn get_connector_schema(&self, connector_id: &str) -> Result<Option<OnboardingSchema>, String>;
    async fn list_connectors(&self) -> Result<Vec<ConnectorInfo>, String>;
}

pub struct GatewayQueryHandler<R: GatewayProfileRepository> {
    repo: R,
    registry: ConnectorRegistry,
}

impl<R: GatewayProfileRepository> GatewayQueryHandler<R> {
    pub fn new(repo: R, registry: ConnectorRegistry) -> Self {
        Self { repo, registry }
    }
}

#[async_trait]
impl<R: GatewayProfileRepository + Send + Sync> QueryHandler for GatewayQueryHandler<R> {
    async fn get_gateway_profile(&self, id: Uuid) -> Result<Option<GatewayProfile>, String> {
        self.repo.load(id).await
    }

    async fn list_gateway_profiles(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, String> {
        self.repo.find_active_for_operator(operator_id).await
    }

    async fn get_connector_schema(&self, connector_id: &str) -> Result<Option<OnboardingSchema>, String> {
        match self.registry.get(connector_id) {
            Ok(connector) => Ok(Some(connector.onboarding_schema())),
            Err(_) => Ok(None),
        }
    }

    async fn list_connectors(&self) -> Result<Vec<ConnectorInfo>, String> {
        let connectors: Vec<&dyn AcquirerConnector> = self.registry.list_all();
        Ok(connectors
            .iter()
            .map(|c| ConnectorInfo {
                connector_id: c.connector_id().to_string(),
                capabilities: format!("{:?}", c.capabilities()),
                settlement_cycle: format!("{:?}", c.settlement_cycle()),
                test_card_count: c.test_card_numbers().len(),
            })
            .collect())
    }
}
