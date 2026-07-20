//! gRPC server implementation for IAM Service.
//!
//! Provides inter-service authentication and authorization via gRPC.

use uuid::Uuid;
use std::sync::Arc;

use crate::application::services::AuthService;
use platform_error::PlatformError;

/// gRPC server implementation wrapping the HTTP service.
pub struct GrpcIamService {
    service: Arc<dyn AuthService>,
}

impl GrpcIamService {
    pub fn new(service: Arc<dyn AuthService>) -> Self {
        Self { service }
    }

    /// Authenticate a user and return tokens.
    pub async fn authenticate(
        &self,
        email: String,
        password: String,
    ) -> Result<(String, String, u64), PlatformError> {
        let cmd = crate::application::commands::AuthenticateCommand {
            email,
            password,
            ip_address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            user_agent: "grpc-service".to_string(),
        };

        let response = self.service.authenticate(cmd).await?;
        Ok((response.access_token, response.refresh_token, response.expires_in))
    }

    /// Validate a permission for a principal.
    pub async fn validate_permission(
        &self,
        principal_id: Uuid,
        resource: String,
        action: String,
    ) -> Result<(bool, bool), PlatformError> {
        let query = crate::application::queries::ValidatePermissionQuery {
            principal_id,
            resource,
            action,
            context: crate::domain::value_objects::PermissionContext {
                amount: None,
                acquirer_link_id: None,
            },
        };

        let result = self.service.validate_permission(query).await?;
        Ok((result.allowed, result.requires_maker_checker))
    }

    /// Create an API key for a principal.
    pub async fn create_api_key(
        &self,
        principal_id: Uuid,
        name: String,
        scopes: Vec<String>,
    ) -> Result<(String, String), PlatformError> {
        let cmd = crate::application::commands::CreateApiKeyCommand {
            principal_id,
            name,
            scopes,
            acquirer_link_ids: None,
            expires_in_days: Some(90),
        };

        let result = self.service.create_api_key(cmd).await?;
        Ok((result.api_key_id, result.api_key_secret))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_iam_service_creation() {
        // Verify module compiles
    }
}
