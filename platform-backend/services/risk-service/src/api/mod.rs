pub mod routes;
use std::sync::Arc;
use crate::application::services::RiskServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<RiskServiceImpl> }
impl AppState { pub fn new(service: RiskServiceImpl) -> Self { Self { service: Arc::new(service) } } }
