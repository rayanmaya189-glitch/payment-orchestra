pub mod dto;
pub mod grpc;
pub mod http;
pub mod routes;

use std::sync::Arc;

use crate::application::services::GatewayServiceImpl;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<GatewayServiceImpl>,
}
