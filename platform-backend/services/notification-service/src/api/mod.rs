pub mod routes;
use std::sync::Arc;
use crate::application::services::NotificationServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<NotificationServiceImpl> }
impl AppState {
    pub fn new(service: NotificationServiceImpl) -> Self { Self { service: Arc::new(service) } }
}
