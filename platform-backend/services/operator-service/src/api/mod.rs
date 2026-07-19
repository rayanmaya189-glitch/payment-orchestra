pub mod dto;
pub mod grpc;
pub mod http;
pub mod routes;

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::application::services::OperatorServiceImpl;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<OperatorServiceImpl>,
}

impl AppState {
    pub fn new(db: DatabaseConnection, service: OperatorServiceImpl) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}
