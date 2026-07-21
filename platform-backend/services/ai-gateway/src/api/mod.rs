pub mod dto;
pub mod routes;
use std::sync::Arc;
use crate::application::services::AiGatewayServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<AiGatewayServiceImpl> }
impl AppState { pub fn new(service: AiGatewayServiceImpl) -> Self { Self { service: Arc::new(service) } } }
