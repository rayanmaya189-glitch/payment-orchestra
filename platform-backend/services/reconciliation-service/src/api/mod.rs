pub mod routes;
use std::sync::Arc;
use crate::application::services::ReconciliationServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<ReconciliationServiceImpl> }
impl AppState { pub fn new(service: ReconciliationServiceImpl) -> Self { Self { service: Arc::new(service) } } }
