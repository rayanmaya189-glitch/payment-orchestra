//! PostgreSQL-backed LinkRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::LinkRepository;
use crate::domain::{LinkError, LinkStatus, MerchantAcquirerLink, HealthStatus, LinkEnvironment};
use crate::entities::{
    ActiveModel as LinkActiveModel,
    Column as LinkColumn,
    Entity as LinkEntity,
    Model as LinkModel,
};

pub struct PostgresLinkRepository {
    pub db: DatabaseConnection,
}

impl PostgresLinkRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LinkRepository for PostgresLinkRepository {
    async fn load(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        let result = LinkEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, link: &MerchantAcquirerLink) -> Result<(), LinkError> {
        let model = domain_to_model(link);
        let exists = LinkEntity::find_by_id(link.link_id)
            .one(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?
            .is_some();

        if exists {
            LinkEntity::update(LinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        } else {
            LinkEntity::insert(LinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let models = LinkEntity::find()
            .filter(LinkColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_active_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let models = LinkEntity::find()
            .filter(LinkColumn::OperatorId.eq(operator_id))
            .filter(LinkColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let models = LinkEntity::find()
            .filter(LinkColumn::ConnectorId.eq(connector_id))
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_credentials_hash(&self, hash: &str) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        let result = LinkEntity::find()
            .filter(LinkColumn::CredentialsHash.eq(hash))
            .one(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_expired_credentials(&self) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let models = LinkEntity::find()
            .filter(LinkColumn::Status.eq("credentials_expired"))
            .all(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn count_active_by_connector(&self, operator_id: Uuid, connector_id: &str) -> Result<usize, LinkError> {
        let count = LinkEntity::find()
            .filter(
                sea_orm::Condition::all()
                    .add(LinkColumn::OperatorId.eq(operator_id))
                    .add(LinkColumn::ConnectorId.eq(connector_id))
                    .add(LinkColumn::Status.eq("active")),
            )
            .count(&self.db)
            .await
            .map_err(|e| LinkError::InvalidRequest(e.to_string()))?;
        Ok(count as usize)
    }
}

// ─── Domain ↔ Model conversion ───────────────────────────────────────────────

fn domain_to_model(l: &MerchantAcquirerLink) -> LinkModel {
    LinkModel {
        link_id: l.link_id,
        operator_id: l.operator_id,
        connector_id: l.connector_id.clone(),
        display_name: l.display_name.clone(),
        environment: l.environment.as_str().to_string(),
        encrypted_credentials: l.encrypted_credentials.clone(),
        credentials_hash: l.credentials_hash.clone(),
        status: l.status.as_str().to_string(),
        health_status: l.health_status.as_str().to_string(),
        last_tested_at: l.last_tested_at,
        last_healthy_at: l.last_healthy_at,
        credentials_expires_at: l.credentials_expires_at,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }
}

fn model_to_domain(m: LinkModel) -> Result<MerchantAcquirerLink, LinkError> {
    let environment = LinkEnvironment::parse_str(&m.environment)
        .ok_or_else(|| LinkError::InvalidRequest(format!("Unknown environment: {}", m.environment)))?;
    let status = LinkStatus::parse_str(&m.status)
        .ok_or_else(|| LinkError::InvalidRequest(format!("Unknown status: {}", m.status)))?;
    let health_status = HealthStatus::parse_str(&m.health_status)
        .ok_or_else(|| LinkError::InvalidRequest(format!("Unknown health_status: {}", m.health_status)))?;

    Ok(MerchantAcquirerLink {
        link_id: m.link_id,
        operator_id: m.operator_id,
        connector_id: m.connector_id,
        display_name: m.display_name,
        environment,
        encrypted_credentials: m.encrypted_credentials,
        credentials_hash: m.credentials_hash,
        status,
        health_status,
        last_tested_at: m.last_tested_at,
        last_healthy_at: m.last_healthy_at,
        credentials_expires_at: m.credentials_expires_at,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}
