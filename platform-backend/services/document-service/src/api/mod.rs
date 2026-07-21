pub mod routes;
pub mod dto;

use std::sync::Arc;
use crate::application::services::DocumentServiceImpl;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<DocumentServiceImpl>,
}

impl AppState {
    pub fn new(service: DocumentServiceImpl) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}
