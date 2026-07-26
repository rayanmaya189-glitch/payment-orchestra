//! gRPC service implementation for the API Gateway management API.
//! Provides operational RPCs for inspecting routes and request logs.
//! The core gateway function (HTTP-to-gRPC routing) is handled by the
//! HTTP ingress layer; this gRPC service exposes internal management.

use uuid::Uuid;

use crate::api::GatewayApi;
use crate::domain::GatewayError;
use crate::queries::types::GatewayHealth;

// ─── Gateway Management Service ─────────────────────────────────────────────
//
// The api-gateway is primarily an HTTP ingress. This internal gRPC service
// provides operational RPCs. Use cases:
//   - Internal service mesh probing: GetHealth, ListRoutes
//   - Debugging: GetRequestLog
//   - Observability: Route count, uptime

pub struct GatewayGrpcService {
    api: GatewayApi,
}

impl GatewayGrpcService {
    pub fn new(api: GatewayApi) -> Self {
        Self { api }
    }

    /// List all registered routes.
    pub async fn list_routes(&self) -> Result<Vec<String>, GatewayError> {
        let routes = self.api.list_routes().await?;
        Ok(routes.into_iter().map(|r| {
            format!("{} {} -> {}::{}", r.http_method, r.url_pattern, r.grpc_service, r.grpc_method)
        }).collect())
    }

    /// Get the current health status.
    pub async fn health_check(&self) -> GatewayHealth {
        self.api.health_check().await
    }

    /// Get a request log entry by ID.
    pub async fn get_request(&self, request_id: Uuid) -> Result<crate::domain::ProcessedRequest, GatewayError> {
        self.api.get_request(request_id).await
    }
}
