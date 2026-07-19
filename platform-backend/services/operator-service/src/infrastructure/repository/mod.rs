use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::Operator;
use platform_error::PlatformError;

#[async_trait]
pub trait OperatorRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<Operator>, PlatformError>;
    async fn save(&self, operator: &Operator) -> Result<(), PlatformError>;
    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, PlatformError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, PlatformError>;
    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Operator>, PlatformError>;
    async fn list(&self, status: Option<&str>, limit: u64, offset: u64) -> Result<Vec<Operator>, PlatformError>;
}
