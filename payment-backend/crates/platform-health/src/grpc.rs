//! gRPC Health Checking Protocol implementation.
//!
//! Every service serves this health endpoint to be discoverable via etcd.
//! The implementation complies with the standard gRPC Health Checking Protocol.

use tonic::{Request, Response, Status};

use platform_proto::health::health_server::Health;
use platform_proto::health::{health_check_response, HealthCheckRequest, HealthCheckResponse};

/// A simple health check service that reports SERVING status.
pub struct HealthService {
    service_name: String,
}

impl HealthService {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

#[tonic::async_trait]
impl Health for HealthService {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let reply = HealthCheckResponse {
            status: health_check_response::ServingStatus::Serving as i32,
        };
        Ok(Response::new(reply))
    }
}
