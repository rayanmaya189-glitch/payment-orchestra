pub mod routes;
use std::sync::Arc;
use crate::application::services::DisputeServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<DisputeServiceImpl> }
impl AppState { pub fn new(service: DisputeServiceImpl) -> Self { Self { service: Arc::new(service) } } }
