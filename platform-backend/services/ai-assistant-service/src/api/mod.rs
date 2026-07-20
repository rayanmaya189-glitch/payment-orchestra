pub mod routes;

use std::sync::Arc;
use crate::application::services::AiAssistantServiceImpl;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<AiAssistantServiceImpl>,
}

impl AppState {
    pub fn new(service: AiAssistantServiceImpl) -> Self {
        Self { service: Arc::new(service) }
    }
}
