use async_trait::async_trait;
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::aggregates::Operator;
use platform_error::PlatformError;

#[async_trait]
pub trait OperatorService: Send + Sync {
    async fn register_operator(&self, cmd: RegisterOperatorCommand) -> Result<OperatorResponse, PlatformError>;
    async fn verify_email(&self, cmd: VerifyEmailCommand) -> Result<(), PlatformError>;
    async fn update_status(&self, cmd: UpdateOperatorStatusCommand) -> Result<(), PlatformError>;
    async fn get_operator(&self, query: GetOperatorQuery) -> Result<OperatorResponse, PlatformError>;
    async fn list_operators(&self, query: ListOperatorsQuery) -> Result<Vec<OperatorResponse>, PlatformError>;
}
