pub mod routes;
use std::sync::Arc;
use crate::application::services::SagaServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<SagaServiceImpl> }
impl AppState { pub fn new(service: SagaServiceImpl) -> Self { Self { service: Arc::new(service) } } }
