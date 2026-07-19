use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::domain::aggregates::{KybCase, KybDocument};
use crate::infrastructure::entities::{kyb_case, kyb_document};
use crate::infrastructure::repository::KybCaseRepository;
use platform_error::PlatformError;

pub struct PostgresKybCaseRepository {
    db: DatabaseConnection,
}

impl PostgresKybCaseRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl KybCaseRepository for PostgresKybCaseRepository {
    async fn save(&self, case: &KybCase) -> Result<(), PlatformError> {
        let active_model: kyb_case::ActiveModel = case.clone().into();

        let existing = kyb_case::Entity::find_by_id(case.id)
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

    async fn load(&self, id: Uuid) -> Result<Option<KybCase>, PlatformError> {
        let model = kyb_case::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match model {
            Some(model) => {
                let documents = self.load_documents(id).await?;
                Ok(Some(model.to_domain(documents)))
            }
            None => Ok(None),
        }
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, PlatformError> {
        let model = kyb_case::Entity::find()
            .filter(kyb_case::Column::OperatorId.eq(operator_id))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match model {
            Some(model) => {
                let documents = self.load_documents(model.id).await?;
                Ok(Some(model.to_domain(documents)))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, status: Option<&str>, limit: u64, offset: u64) -> Result<Vec<KybCase>, PlatformError> {
        let mut query = kyb_case::Entity::find();

        if let Some(status) = status {
            query = query.filter(kyb_case::Column::Status.eq(status));
        }

        let models = query
            .order_by_desc(kyb_case::Column::CreatedAt)
            .limit(Some(limit))
            .offset(Some(offset))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        let mut cases = Vec::new();
        for model in models {
            let documents = self.load_documents(model.id).await?;
            cases.push(model.to_domain(documents));
        }

        Ok(cases)
    }

    async fn save_document(&self, document: &KybDocument) -> Result<(), PlatformError> {
        let active_model: kyb_document::ActiveModel = document.clone().into();

        let existing = kyb_document::Entity::find_by_id(document.id)
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

    async fn load_documents(&self, kyb_case_id: Uuid) -> Result<Vec<KybDocument>, PlatformError> {
        let models = kyb_document::Entity::find()
            .filter(kyb_document::Column::KybCaseId.eq(kyb_case_id))
            .order_by_desc(kyb_document::Column::UploadedAt)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }

    async fn update_document_verification(&self, document_id: Uuid, verified: bool) -> Result<(), PlatformError> {
        let model = kyb_document::Entity::find_by_id(document_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        if let Some(model) = model {
            let mut active: kyb_document::ActiveModel = model.into();
            active.verified = sea_orm::Set(verified);
            active.verified_at = sea_orm::Set(Some(chrono::Utc::now().into()));
            active
                .update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
        }

        Ok(())
    }
}
