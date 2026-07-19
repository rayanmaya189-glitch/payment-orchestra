use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, QueryOrder};
use uuid::Uuid;

use crate::domain::aggregates::{ApiKey, Principal, PendingChange};
use crate::domain::entities::RoleAssignment;
use crate::infrastructure::entities::{api_key, pending_change, principal, role_assignment};
use crate::infrastructure::repository::{ApiKeyRepository, PendingChangeRepository, PrincipalRepository};
use platform_error::PlatformError;

// ==================== Principal Repository ====================

pub struct PostgresPrincipalRepository {
    db: DatabaseConnection,
}

impl PostgresPrincipalRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PrincipalRepository for PostgresPrincipalRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Principal>, PlatformError> {
        let model = principal::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Principal>, PlatformError> {
        let model = principal::Entity::find()
            .filter(principal::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, principal: &Principal) -> Result<(), PlatformError> {
        let active_model: principal::ActiveModel = principal.clone().into();

        let existing = principal::Entity::find_by_id(principal.id)
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

    async fn list_roles(&self, principal_id: Uuid) -> Result<Vec<RoleAssignment>, PlatformError> {
        let models = role_assignment::Entity::find()
            .filter(role_assignment::Column::PrincipalId.eq(principal_id))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }
}

// ==================== API Key Repository ====================

pub struct PostgresApiKeyRepository {
    db: DatabaseConnection,
}

impl PostgresApiKeyRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ApiKeyRepository for PostgresApiKeyRepository {
    async fn save(&self, api_key: &ApiKey) -> Result<(), PlatformError> {
        let active_model: api_key::ActiveModel = api_key.clone().into();

        let existing = api_key::Entity::find_by_id(api_key.id)
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

    async fn find_by_key_hash(&self, hash: &[u8]) -> Result<Option<ApiKey>, PlatformError> {
        let model = api_key::Entity::find()
            .filter(api_key::Column::KeyHash.eq(hash))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn revoke(&self, id: Uuid) -> Result<(), PlatformError> {
        let model = api_key::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        if let Some(model) = model {
            let mut active: api_key::ActiveModel = model.into();
            active.revoked_at = sea_orm::Set(Some(chrono::Utc::now().into()));
            active
                .update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
        }

        Ok(())
    }

    async fn list_by_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, PlatformError> {
        let models = api_key::Entity::find()
            .filter(api_key::Column::PrincipalId.eq(principal_id))
            .order_by_desc(api_key::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }
}

// ==================== Pending Change Repository ====================

pub struct PostgresPendingChangeRepository {
    db: DatabaseConnection,
}

impl PostgresPendingChangeRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PendingChangeRepository for PostgresPendingChangeRepository {
    async fn save(&self, change: &PendingChange) -> Result<(), PlatformError> {
        let active_model: pending_change::ActiveModel = change.clone().into();

        let existing = pending_change::Entity::find_by_id(change.change_id)
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

    async fn load(&self, id: Uuid) -> Result<Option<PendingChange>, PlatformError> {
        let model = pending_change::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }
}
