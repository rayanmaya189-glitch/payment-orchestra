//! IAM Service gRPC client — authentication and authorization.
//!
//! Used by API Gateway to verify tokens, authenticate principals,
//! and check ABAC permissions.

use platform_proto::iam::iam_service_client::IamServiceClient;
use platform_proto::iam::{
    AuthenticateRequest, AuthenticateResponse,
    ValidatePermissionRequest, ValidatePermissionResponse,
    GetPrincipalRequest, GetPrincipalResponse,
};

use crate::client::{ClientError, ServiceConnection};

/// Client for the IAM service (BC-02).
#[derive(Debug, Clone)]
pub struct IamClient {
    conn: ServiceConnection,
}

impl IamClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("iam-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("iam-service", addr)?;
        Ok(Self { conn })
    }

    /// Authenticate a principal with email + password.
    pub async fn authenticate(
        &self,
        email: String,
        password: String,
        ip_address: String,
        user_agent: String,
    ) -> Result<AuthenticateResponse, ClientError> {
        let mut client = IamServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(AuthenticateRequest {
            email,
            password,
            ip_address,
            user_agent,
        });
        let response = client
            .authenticate(request)
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "iam-service".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }

    /// Validate a principal's permission using ABAC.
    pub async fn validate_permission(
        &self,
        principal_id: String,
        resource: String,
        action: String,
        amount_minor_units: i64,
        currency_code: String,
    ) -> Result<ValidatePermissionResponse, ClientError> {
        let mut client = IamServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(ValidatePermissionRequest {
            principal_id,
            resource,
            action,
            amount_minor_units,
            currency_code,
        });
        let response = client
            .validate_permission(request)
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "iam-service".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }

    /// Get a principal by ID.
    pub async fn get_principal(
        &self,
        principal_id: String,
    ) -> Result<GetPrincipalResponse, ClientError> {
        let mut client = IamServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(GetPrincipalRequest { principal_id });
        let response = client
            .get_principal(request)
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "iam-service".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }
}
