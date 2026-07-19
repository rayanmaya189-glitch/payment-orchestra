use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::domain::aggregates::Operator;
use crate::infrastructure::repository::OperatorRepository;
use platform_error::PlatformError;

pub struct PostgresOperatorRepository {
    db: DatabaseConnection,
}

impl PostgresOperatorRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OperatorRepository for PostgresOperatorRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Operator>, PlatformError> {
        let _ = (&self.db, id);
        Ok(None)
    }

    async fn save(&self, operator: &Operator) -> Result<(), PlatformError> {
        let _ = (&self.db, operator);
        Ok(())
    }

    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, PlatformError> {
        let _ = (&self.db, license);
        Ok(None)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, PlatformError> {
        let _ = (&self.db, email);
        Ok(None)
    }

    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Operator>, PlatformError> {
        let _ = (&self.db, subdomain);
        Ok(None)
    }
}
