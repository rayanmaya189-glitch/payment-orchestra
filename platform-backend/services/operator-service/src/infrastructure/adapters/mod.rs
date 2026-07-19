use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::domain::aggregates::Operator;
use crate::infrastructure::entities::operator;
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
        let model = operator::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, operator: &Operator) -> Result<(), PlatformError> {
        let active_model: operator::ActiveModel = operator.clone().into();

        // Try update first, insert if not found
        let existing = operator::Entity::find_by_id(operator.id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }

    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, PlatformError> {
        let model = operator::Entity::find()
            .filter(operator::Column::TradeLicenseNo.eq(license))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, PlatformError> {
        let model = operator::Entity::find()
            .filter(operator::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Operator>, PlatformError> {
        let model = operator::Entity::find()
            .filter(operator::Column::Subdomain.eq(subdomain))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn list(
        &self,
        status: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Operator>, PlatformError> {
        let mut query = operator::Entity::find();

        if let Some(status) = status {
            query = query.filter(operator::Column::Status.eq(status));
        }

        let models = query
            .order_by_desc(operator::Column::CreatedAt)
            .limit(Some(limit))
            .offset(Some(offset))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }
}
