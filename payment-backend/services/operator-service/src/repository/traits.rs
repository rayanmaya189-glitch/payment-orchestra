//! Repository trait for Operator aggregate.

use uuid::Uuid;

use crate::domain::{Operator, OperatorError};

/// Repository trait for Operator aggregate
#[async_trait::async_trait]
pub trait OperatorRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<Operator>, OperatorError>;
    async fn save(&self, operator: &mut Operator) -> Result<(), OperatorError>;
    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, OperatorError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, OperatorError>;
    #[allow(dead_code)]
    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Operator>, OperatorError>;
    async fn list_by_status(&self, status: Option<&str>) -> Result<Vec<Operator>, OperatorError>;
}
