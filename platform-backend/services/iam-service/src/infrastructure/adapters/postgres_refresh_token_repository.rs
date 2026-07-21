use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::domain::aggregates::{RefreshToken, RefreshTokenStatus};
use crate::domain::rules::RefreshTokenRepository;
use crate::infrastructure::entities::refresh_token_entity;
use platform_error::PlatformError;

pub struct PostgresRefreshTokenRepository {
    db: DatabaseConnection,
}

impl PostgresRefreshTokenRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RefreshTokenRepository for PostgresRefreshTokenRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RefreshToken>, PlatformError> {
        let model = refresh_token_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(model.map(|m| m.into()))
    }

    async fn save(&self, token: &RefreshToken) -> Result<(), PlatformError> {
        let existing = refresh_token_entity::Entity::find_by_id(token.token_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        if let Some(model) = existing {
            let mut active = refresh_token_entity::ActiveModel::from(model);
            active.status = Set(token.status.as_str().to_string());
            active.revoked_at = Set(token.revoked_at.map(|dt| dt.into()));
            active.revoke_reason = Set(token.revoke_reason.clone());

            active.update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB update failed: {e}")))?;
        } else {
            let active = refresh_token_entity::ActiveModel {
                id: Set(token.token_id),
                principal_id: Set(token.principal_id),
                role: Set(token.role.clone()),
                client_fingerprint: Set(token.client_fingerprint.clone()),
                status: Set(token.status.as_str().to_string()),
                created_at: Set(token.created_at.into()),
                revoked_at: Set(token.revoked_at.map(|dt| dt.into())),
                revoke_reason: Set(token.revoke_reason.clone()),
            };

            active.insert(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB insert failed: {e}")))?;
        }

        Ok(())
    }

    async fn revoke_all_for_principal(&self, principal_id: Uuid) -> Result<(), PlatformError> {
        let tokens = refresh_token_entity::Entity::find()
            .filter(refresh_token_entity::Column::PrincipalId.eq(principal_id))
            .filter(refresh_token_entity::Column::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        for token in tokens {
            let mut active = refresh_token_entity::ActiveModel::from(token);
            active.status = Set("revoked".to_string());
            active.revoked_at = Set(Some(chrono::Utc::now().into()));
            active.revoke_reason = Set(Some("bulk_revoke".to_string()));

            active.update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB update failed: {e}")))?;
        }

        Ok(())
    }
}

impl From<refresh_token_entity::Model> for RefreshToken {
    fn from(m: refresh_token_entity::Model) -> Self {
        RefreshToken {
            token_id: m.id,
            principal_id: m.principal_id,
            role: m.role,
            client_fingerprint: m.client_fingerprint,
            status: match m.status.as_str() {
                "revoked" => RefreshTokenStatus::Revoked,
                _ => RefreshTokenStatus::Active,
            },
            created_at: m.created_at.into(),
            revoked_at: m.revoked_at.map(|dt| dt.into()),
            revoke_reason: m.revoke_reason,
        }
    }
}
