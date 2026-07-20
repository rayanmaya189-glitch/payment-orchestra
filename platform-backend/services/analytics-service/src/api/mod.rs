pub mod routes;
use std::sync::Arc;
use crate::application::services::AnalyticsServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<AnalyticsServiceImpl> }
impl AppState { pub fn new(service: AnalyticsServiceImpl) -> Self { Self { service: Arc::new(service) } } }
