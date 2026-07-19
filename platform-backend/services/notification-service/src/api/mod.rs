#![allow(dead_code)]
pub mod dto; pub mod routes;
use std::sync::Arc; use crate::application::services::NotificationServiceImpl;
#[derive(Clone)] pub struct AppState { pub service: Arc<NotificationServiceImpl> }
