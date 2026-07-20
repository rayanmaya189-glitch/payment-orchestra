pub mod routes;
use std::sync::Arc;
use crate::application::services::ComplianceServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<ComplianceServiceImpl> }
impl AppState {
    pub fn new(service: ComplianceServiceImpl) -> Self { Self { service: Arc::new(service) } }
}
