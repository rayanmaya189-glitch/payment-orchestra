pub mod dto;
pub mod grpc;
pub mod http;
pub mod routes;

use std::sync::Arc;

use crate::application::services::{RiskServiceImpl, RiskService};

/// Shared application state injected into Axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub service: Arc<dyn RiskService>,
}

impl AppState {
    pub fn new(service: RiskServiceImpl) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}
