use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::domain::aggregates::ApiKey;
use crate::domain::rules::ApiKeyRepository;
use crate::infrastructure::entities::{api_key_entity, principal_entity};
use platform_error::PlatformError;

pub struct PostgresApiKeyRepository {
    db: DatabaseConnection,
}

impl PostgresApiKeyRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// Implementation of the middleware's ApiKeyLookup trait for database-backed validation.
#[async_trait]
impl platform_middleware::auth::ApiKeyLookup for PostgresApiKeyRepository {
    async fn find_principal_by_key_hash(&self, key_hash: &[u8]) -> Result<Option<(Uuid, String)>, String> {
        // Find the API key by hash
        let api_key_model = api_key_entity::Entity::find()
            .filter(api_key_entity::Column::KeyHash.eq(key_hash))
            .one(&self.db)
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;

        let api_key = match api_key_model {
            Some(m) => m,
            None => return Ok(None),
        };

        // Check not revoked
        if api_key.revoked_at.is_some() {
            return Ok(None);
        }

        // Check not expired
        let now = chrono::Utc::now();
        if api_key.expires_at < now {
            return Ok(None);
        }

        // Find the associated principal to get the role
        let principal_model = principal_entity::Entity::find_by_id(api_key.principal_id)
            .one(&self.db)
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;

        let principal = match principal_model {
            Some(m) => m,
            None => return Ok(None),
        };

        // Check principal is active
        if principal.status != "active" {
            return Ok(None);
        }

        // Derive role from principal_type (api_client principals get api_client role)
        let role = match principal.principal_type.as_str() {
            "api_client" => "api_client".to_string(),
            "system" => "api_client".to_string(),
            _ => {
                // For user-type principals, use a default role based on scopes
                let scopes: Vec<String> = serde_json::from_value(api_key.scopes.into())
                    .unwrap_or_default();
                if scopes.contains(&"admin".to_string()) {
                    "operator_admin".to_string()
                } else {
                    "api_client".to_string()
                }
            }
        };

        Ok(Some((api_key.principal_id, role)))
    }
}

#[async_trait]
impl ApiKeyRepository for PostgresApiKeyRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ApiKey>, PlatformError> {
        let model = api_key_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(model.map(|m| m.into()))
    }

    async fn find_by_key_hash(&self, key_hash: &[u8]) -> Result<Option<ApiKey>, PlatformError> {
        let model = api_key_entity::Entity::find()
            .filter(api_key_entity::Column::KeyHash.eq(key_hash))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(model.map(|m| m.into()))
    }

    async fn save(&self, api_key: &ApiKey) -> Result<(), PlatformError> {
        let existing = api_key_entity::Entity::find_by_id(api_key.api_key_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        if let Some(model) = existing {
            let mut active = api_key_entity::ActiveModel::from(model);
            active.name = Set(api_key.name.clone());
            active.scopes = Set(serde_json::to_value(&api_key.scopes).unwrap().into());
            active.revoked_at = Set(api_key.revoked_at.map(|dt| dt.into()));

            active.update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB update failed: {e}")))?;
        } else {
            let active = api_key_entity::ActiveModel {
                id: Set(api_key.api_key_id),
                principal_id: Set(api_key.principal_id),
                name: Set(api_key.name.clone()),
                key_hash: Set(api_key.key_hash.clone()),
                scopes: Set(serde_json::to_value(&api_key.scopes).unwrap().into()),
                acquirer_link_ids: Set(api_key.acquirer_link_ids.as_ref().map(|ids| {
                    serde_json::to_value(ids).unwrap().into()
                })),
                expires_at: Set(api_key.expires_at.into()),
                revoked_at: Set(api_key.revoked_at.map(|dt| dt.into())),
                created_at: Set(api_key.created_at.into()),
            };

            active.insert(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB insert failed: {e}")))?;
        }

        Ok(())
    }

    async fn list_by_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, PlatformError> {
        let models = api_key_entity::Entity::find()
            .filter(api_key_entity::Column::PrincipalId.eq(principal_id))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }
}

impl From<api_key_entity::Model> for ApiKey {
    fn from(m: api_key_entity::Model) -> Self {
        let scopes: Vec<String> = serde_json::from_value(m.scopes.into()).unwrap_or_default();
        let acquirer_link_ids: Option<Vec<Uuid>> = m.acquirer_link_ids
            .and_then(|ids| serde_json::from_value(ids.into()).ok());

        ApiKey {
            api_key_id: m.id,
            principal_id: m.principal_id,
            name: m.name,
            key_hash: m.key_hash,
            key_prefix: String::new(),
            scopes,
            acquirer_link_ids,
            expires_at: m.expires_at.into(),
            revoked_at: m.revoked_at.map(|dt| dt.into()),
            created_at: m.created_at.into(),
        }
    }
}
