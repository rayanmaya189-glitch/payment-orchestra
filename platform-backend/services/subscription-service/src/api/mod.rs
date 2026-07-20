pub mod routes;

use std::sync::Arc;
use crate::application::services::SubscriptionServiceImpl;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<SubscriptionServiceImpl>,
}

impl AppState {
    pub fn new(service: SubscriptionServiceImpl) -> Self {
        Self { service: Arc::new(service) }
    }
}
