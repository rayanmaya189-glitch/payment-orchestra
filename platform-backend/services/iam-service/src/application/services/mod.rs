use async_trait::async_trait;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::value_objects::PermissionResult;
use platform_error::PlatformError;

#[derive(Debug)]
pub struct AuthResult {
    pub access_token: String,
    pub refresh_token: String,
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn authenticate(&self, cmd: AuthenticateCommand) -> Result<AuthResult, PlatformError>;
    async fn issue_token(&self, cmd: IssueTokenCommand) -> Result<AuthResult, PlatformError>;
    async fn validate_permission(&self, query: ValidatePermissionQuery) -> Result<PermissionResult, PlatformError>;
    async fn create_api_key(&self, cmd: CreateApiKeyCommand) -> Result<ApiKeyResult, PlatformError>;
    async fn revoke_api_key(&self, cmd: RevokeApiKeyCommand) -> Result<(), PlatformError>;
    async fn approve_pending_change(&self, cmd: ApprovePendingChangeCommand) -> Result<(), PlatformError>;
}

#[derive(Debug)]
pub struct ApiKeyResult {
    pub api_key_id: String,
    pub api_key_secret: String,
}
