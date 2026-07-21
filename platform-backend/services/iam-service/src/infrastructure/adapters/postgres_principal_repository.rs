use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::domain::aggregates::Principal;
use crate::domain::rules::PrincipalRepository;
use crate::infrastructure::entities::principal_entity;
use platform_error::PlatformError;

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
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Principal>, PlatformError> {
        let model = principal_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(model.map(|m| m.into()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Principal>, PlatformError> {
        let model = principal_entity::Entity::find()
            .filter(principal_entity::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(model.map(|m| m.into()))
    }

    async fn save(&self, principal: &Principal) -> Result<(), PlatformError> {
        let existing = principal_entity::Entity::find_by_id(principal.principal_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        if let Some(model) = existing {
            let mut active = principal_entity::ActiveModel::from(model);
            active.principal_type = Set(principal.principal_type.as_str().to_string());
            active.email = Set(principal.email.as_ref().map(|e| e.to_string()));
            active.password_hash = Set(principal.password_hash.clone());
            active.mfa_enrolled = Set(principal.mfa_enrolled);
            active.mfa_method = Set(principal.mfa_method.as_ref().map(|m| m.as_str().to_string()));
            active.status = Set(principal.status.as_str().to_string());
            active.role = Set(principal.role.as_str().to_string());
            active.failed_login_attempts = Set(principal.failed_login_attempts);
            active.locked_until = Set(principal.locked_until.map(|dt| dt.into()));
            active.last_login_at = Set(principal.last_login_at.map(|dt| dt.into()));

            active.update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB update failed: {e}")))?;
        } else {
            let active = principal_entity::ActiveModel {
                id: Set(principal.principal_id),
                principal_type: Set(principal.principal_type.as_str().to_string()),
                email: Set(principal.email.as_ref().map(|e| e.to_string())),
                password_hash: Set(principal.password_hash.clone()),
                mfa_enrolled: Set(principal.mfa_enrolled),
                mfa_method: Set(principal.mfa_method.as_ref().map(|m| m.as_str().to_string())),
                status: Set(principal.status.as_str().to_string()),
                role: Set(principal.role.as_str().to_string()),
                failed_login_attempts: Set(principal.failed_login_attempts),
                locked_until: Set(principal.locked_until.map(|dt| dt.into())),
                created_at: Set(principal.created_at.into()),
                last_login_at: Set(principal.last_login_at.map(|dt| dt.into())),
            };

            active.insert(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB insert failed: {e}")))?;
        }

        Ok(())
    }

    async fn exists_by_email(&self, email: &str) -> Result<bool, PlatformError> {
        let models = principal_entity::Entity::find()
            .filter(principal_entity::Column::Email.eq(email))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(!models.is_empty())
    }
}

impl From<principal_entity::Model> for Principal {
    fn from(m: principal_entity::Model) -> Self {
        use crate::domain::value_objects::{Email, MfaMethod, PrincipalRole, PrincipalStatus};
        use crate::domain::aggregates::PrincipalType;

        Principal {
            principal_id: m.id,
            principal_type: PrincipalType::from_str(&m.principal_type),
            email: m.email.and_then(|e| Email::new(&e).ok()),
            password_hash: m.password_hash,
            mfa_enrolled: m.mfa_enrolled,
            mfa_method: m.mfa_method.as_ref().map(|s| MfaMethod::from_str(s)),
            mfa_secret: None,
            status: PrincipalStatus::from_str(&m.status),
            role: PrincipalRole::from_str(&m.role),
            failed_login_attempts: m.failed_login_attempts,
            locked_until: m.locked_until.map(|dt| dt.into()),
            created_at: m.created_at.into(),
            last_login_at: m.last_login_at.map(|dt| dt.into()),
            uncommitted_events: Vec::new(),
        }
    }
}
