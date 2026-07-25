//! PostgreSQL-backed MerchantAcquirerLink repository using SeaORM CRUD.
//!
//! Converts between the domain MerchantAcquirerLink model (with LinkEnvironment,
//! LinkStatus, HealthStatus enums via as_str/parse_str) and the flat SeaORM entity.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::link::{
    ActiveModel as LinkActiveModel, Column as LinkColumn,
    Entity as LinkEntity, Model as LinkModel,
};

/// PostgreSQL-backed repository implementing LinkRepository.
#[derive(Clone)]
pub struct PostgresLinkRepository {
    pub db: sea_orm::DatabaseConnection,
}

impl PostgresLinkRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn link_domain_to_model(link: &MerchantAcquirerLink) -> LinkModel {
    LinkModel {
        link_id: link.link_id,
        operator_id: link.operator_id,
        connector_id: link.connector_id.clone(),
        display_name: link.display_name.clone(),
        environment: link.environment.as_str().to_string(),
        encrypted_credentials: link.encrypted_credentials.clone(),
        credentials_hash: link.credentials_hash.clone(),
        status: link.status.as_str().to_string(),
        health_status: link.health_status.as_str().to_string(),
        last_tested_at: link.last_tested_at,
        last_healthy_at: link.last_healthy_at,
        credentials_expires_at: link.credentials_expires_at,
        created_at: link.created_at,
        updated_at: link.updated_at,
    }
}

fn link_model_to_domain(m: LinkModel) -> Result<MerchantAcquirerLink, LinkError> {
    Ok(MerchantAcquirerLink {
        link_id: m.link_id,
        operator_id: m.operator_id,
        connector_id: m.connector_id,
        display_name: m.display_name,
        environment: LinkEnvironment::parse_str(&m.environment)
            .ok_or_else(|| LinkError::InvalidRequest(format!("Invalid environment: {}", m.environment)))?,
        encrypted_credentials: m.encrypted_credentials,
        credentials_hash: m.credentials_hash,
        status: LinkStatus::parse_str(&m.status)
            .ok_or_else(|| LinkError::InvalidRequest(format!("Invalid status: {}", m.status)))?,
        health_status: HealthStatus::parse_str(&m.health_status)
            .ok_or_else(|| LinkError::InvalidRequest(format!("Invalid health_status: {}", m.health_status)))?,
        last_tested_at: m.last_tested_at,
        last_healthy_at: m.last_healthy_at,
        credentials_expires_at: m.credentials_expires_at,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

// ─── LinkRepository Trait Implementation ─────────────────────────────────────

use crate::repository::LinkRepository;

#[async_trait]
impl LinkRepository for PostgresLinkRepository {
    async fn load(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        let result = LinkEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(link_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, link: &MerchantAcquirerLink) -> Result<(), LinkError> {
        let model = link_domain_to_model(link);

        let exists = LinkEntity::find_by_id(link.link_id)
            .one(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?
            .is_some();

        if exists {
            LinkEntity::update(LinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;
        } else {
            LinkEntity::insert(LinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;
        }

        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let results = LinkEntity::find()
            .filter(LinkColumn::OperatorId.eq(operator_id))
            .order_by_asc(LinkColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;

        results.into_iter().map(link_model_to_domain).collect()
    }

    async fn find_active_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let results = LinkEntity::find()
            .filter(LinkColumn::OperatorId.eq(operator_id))
            .filter(LinkColumn::Status.eq("active"))
            .order_by_asc(LinkColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;

        results.into_iter().map(link_model_to_domain).collect()
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let results = LinkEntity::find()
            .filter(LinkColumn::ConnectorId.eq(connector_id))
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;

        results.into_iter().map(link_model_to_domain).collect()
    }

    async fn find_by_credentials_hash(&self, hash: &str) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        let result = LinkEntity::find()
            .filter(LinkColumn::CredentialsHash.eq(hash))
            .one(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(link_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_expired_credentials(&self) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        use chrono::Utc;

        let results = LinkEntity::find()
            .filter(LinkColumn::CredentialsExpiresAt.is_not_null())
            .filter(LinkColumn::CredentialsExpiresAt.lt(Utc::now()))
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;

        results.into_iter().map(link_model_to_domain).collect()
    }

    async fn count_active_by_connector(
        &self,
        operator_id: Uuid,
        connector_id: &str,
    ) -> Result<usize, LinkError> {
        let count = LinkEntity::find()
            .filter(LinkColumn::OperatorId.eq(operator_id))
            .filter(LinkColumn::ConnectorId.eq(connector_id))
            .filter(LinkColumn::Status.eq("active"))
            .count(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(format!("Database error: {e}")))?;

        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_link() -> MerchantAcquirerLink {
        MerchantAcquirerLink::new(
            Uuid::now_v7(),
            "checkout_com".into(),
            "Test Gateway".into(),
            LinkEnvironment::Production,
            vec![1, 2, 3, 4],
            "test_hash_123".into(),
        )
    }

    #[test]
    fn test_link_domain_entity_roundtrip() {
        let link = sample_link();

        let model = link_domain_to_model(&link);
        let roundtrip = link_model_to_domain(model).unwrap();

        assert_eq!(roundtrip.link_id, link.link_id);
        assert_eq!(roundtrip.operator_id, link.operator_id);
        assert_eq!(roundtrip.connector_id, "checkout_com");
        assert_eq!(roundtrip.display_name, "Test Gateway");
        assert_eq!(roundtrip.environment, LinkEnvironment::Production);
        assert_eq!(roundtrip.status, LinkStatus::Testing);
        assert_eq!(roundtrip.health_status, HealthStatus::Unknown);
        assert_eq!(roundtrip.credentials_hash, "test_hash_123");
        assert!(roundtrip.last_tested_at.is_none());
    }

    #[test]
    fn test_enum_serialization_consistency() {
        assert_eq!(LinkEnvironment::Sandbox.as_str(), "sandbox");
        assert_eq!(LinkEnvironment::Production.as_str(), "production");
        assert_eq!(LinkStatus::Active.as_str(), "active");
        assert_eq!(LinkStatus::CredentialsExpired.as_str(), "credentials_expired");
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Unreachable.as_str(), "unreachable");

        assert_eq!(LinkEnvironment::parse_str("sandbox"), Some(LinkEnvironment::Sandbox));
        assert_eq!(LinkStatus::parse_str("active"), Some(LinkStatus::Active));
        assert_eq!(HealthStatus::parse_str("healthy"), Some(HealthStatus::Healthy));
    }
}
