#![allow(dead_code)]
pub mod dto;
pub mod grpc;
pub mod http;
pub mod routes;

use std::sync::Arc;

use crate::application::services::AuthServiceImpl;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<AuthServiceImpl>,
}
