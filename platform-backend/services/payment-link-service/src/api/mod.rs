pub mod routes;
use std::sync::Arc;
use crate::application::services::PaymentLinkServiceImpl;
#[derive(Clone)]
pub struct AppState { pub service: Arc<PaymentLinkServiceImpl> }
impl AppState { pub fn new(service: PaymentLinkServiceImpl) -> Self { Self { service: Arc::new(service) } } }
