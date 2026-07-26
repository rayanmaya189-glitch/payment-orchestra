//! API Gateway public API

pub mod grpc;

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

pub struct GatewayApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl GatewayApi {
    pub fn new(ch: Box<dyn CommandHandler>, qh: Box<dyn QueryHandler>) -> Self {
        Self { command_handler: ch, query_handler: qh }
    }

    pub async fn process_request(&self, cmd: ProcessInboundRequest) -> Result<ProcessedRequest, GatewayError> {
        self.command_handler.process_request(cmd).await
    }
    pub async fn register_route(&self, cmd: RegisterRoute) -> Result<(), GatewayError> {
        self.command_handler.register_route(cmd).await
    }
    pub async fn get_route(&self, method: &str, path: &str) -> Result<RouteDefinition, GatewayError> {
        self.query_handler.get_route(method, path).await
    }
    pub async fn list_routes(&self) -> Result<Vec<RouteDefinition>, GatewayError> {
        self.query_handler.list_routes().await
    }
    pub async fn get_request(&self, request_id: Uuid) -> Result<ProcessedRequest, GatewayError> {
        self.query_handler.get_request(request_id).await
    }
    pub async fn health_check(&self) -> GatewayHealth {
        self.query_handler.health_check().await
    }
}
