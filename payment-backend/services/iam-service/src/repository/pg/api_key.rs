//! PostgreSQL-backed ApiKey operations using SeaORM CRUD.
//!
//! Converts between the domain ApiKey model (with ApiKeyStatus enum, Vec<String> scopes)
//! and the flat SeaORM entity model (api_keys table with JSONB scopes).

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::{ApiKey, ApiKeyStatus, IamError};
use crate::entities::api_key::{
    ActiveModel as ApiKeyActiveModel, Column as ApiKeyColumn,
    Entity as ApiKeyEntity, Model as ApiKeyModel,
};
use super::PostgresIamRepository;

impl PostgresIamRepository {
    /// Load an ApiKey from DB and convert to domain model.
    pub(super) async fn load_api_key_domain(&self, id: Uuid) -> Result<Option<ApiKey>, IamError> {
        let result = ApiKeyEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(api_key_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    /// Save a domain ApiKey to DB.
    pub(super) async fn save_api_key_domain(&self, key: &ApiKey) -> Result<(), IamError> {
        let model = api_key_domain_to_model(key)?;

        let exists = ApiKeyEntity::find_by_id(key.api_key_id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?
            .is_some();

        if exists {
            ApiKeyEntity::update(ApiKeyActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;
        } else {
            ApiKeyEntity::insert(ApiKeyActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;
        }
        Ok(())
    }

    /// List all API keys for a principal.
    pub(super) async fn list_api_keys_for_principal_domain(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError> {
        let models = ApiKeyEntity::find()
            .filter(ApiKeyColumn::PrincipalId.eq(principal_id))
            .all(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;

        models.into_iter().map(api_key_model_to_domain).collect()
    }

    /// Find an API key by name for a principal.
    pub(super) async fn find_api_key_by_name_domain(&self, principal_id: Uuid, name: &str) -> Result<Option<ApiKey>, IamError> {
        let result = ApiKeyEntity::find()
            .filter(ApiKeyColumn::PrincipalId.eq(principal_id))
            .filter(ApiKeyColumn::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(api_key_model_to_domain(model)?)),
            None => Ok(None),
        }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn api_key_domain_to_model(key: &ApiKey) -> Result<ApiKeyModel, IamError> {
    let scopes_json = serde_json::to_value(&key.scopes)
        .map_err(|e| IamError::InvalidRequest(format!("Serialize scopes: {}", e)))?;

    Ok(ApiKeyModel {
        api_key_id: key.api_key_id,
        principal_id: key.principal_id,
        name: key.name.clone(),
        key_hash: key.key_hash.clone(),
        scopes: scopes_json,
        status: key.status.as_str().to_string(),
        created_at: key.created_at,
        expires_at: key.expires_at,
        last_used_at: key.last_used_at,
    })
}

fn api_key_model_to_domain(m: ApiKeyModel) -> Result<ApiKey, IamError> {
    let scopes: Vec<String> = serde_json::from_value(m.scopes)
        .map_err(|e| IamError::InvalidRequest(format!("Deserialize scopes: {}", e)))?;

    Ok(ApiKey {
        api_key_id: m.api_key_id,
        principal_id: m.principal_id,
        name: m.name,
        key_hash: m.key_hash,
        scopes,
        status: ApiKeyStatus::parse_str(&m.status).unwrap_or(ApiKeyStatus::Active),
        created_at: m.created_at,
        expires_at: m.expires_at,
        last_used_at: m.last_used_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_api_key_domain_entity_roundtrip() {
        let now = Utc::now();
        let key = ApiKey {
            api_key_id: Uuid::now_v7(),
            principal_id: Uuid::now_v7(),
            name: "Production Key".into(),
            key_hash: vec![1, 2, 3, 4],
            scopes: vec!["payments:read".into(), "payments:write".into()],
            status: ApiKeyStatus::Active,
            created_at: now,
            expires_at: None,
            last_used_at: None,
        };

        let model = api_key_domain_to_model(&key).unwrap();
        let roundtrip = api_key_model_to_domain(model).unwrap();

        assert_eq!(roundtrip.api_key_id, key.api_key_id);
        assert_eq!(roundtrip.name, key.name);
        assert_eq!(roundtrip.status.as_str(), key.status.as_str());
        assert_eq!(roundtrip.scopes.len(), 2);
        assert_eq!(roundtrip.scopes[0], "payments:read");
        assert!(roundtrip.key_hash.len() > 0);
    }
}
