pub mod routes;

use std::sync::Arc;
use platform_config::AuthConfig;

use crate::application::services::IamServiceImpl;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<IamServiceImpl>,
    pub auth_config: AuthConfig,
}

impl AppState {
    pub fn new(service: IamServiceImpl, auth_config: AuthConfig) -> Self {
        Self {
            service: Arc::new(service),
            auth_config,
        }
    }
}
