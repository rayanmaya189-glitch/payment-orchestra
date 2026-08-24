//! API Gateway repository trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait GatewayRepository: Send + Sync {
    async fn get_route(&self, method: &str, path: &str) -> Result<Option<RouteDefinition>, GatewayError>;
    async fn list_routes(&self) -> Result<Vec<RouteDefinition>, GatewayError>;
    async fn check_rate_limit(&self, key: &str, config: &RateLimitConfig) -> Result<bool, GatewayError>;
    async fn validate_api_key(&self, api_key: &str) -> Result<AuthResult, GatewayError>;
    async fn log_request(&self, request: &ProcessedRequest) -> Result<(), GatewayError>;
    async fn get_request_log(&self, request_id: Uuid) -> Result<Option<ProcessedRequest>, GatewayError>;
}
