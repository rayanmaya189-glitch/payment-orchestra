pub mod routes;
use std::sync::Arc;
use crate::application::services::ApiGatewayServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<ApiGatewayServiceImpl> }
impl AppState { pub fn new(service: ApiGatewayServiceImpl) -> Self { Self { service: Arc::new(service) } } }
