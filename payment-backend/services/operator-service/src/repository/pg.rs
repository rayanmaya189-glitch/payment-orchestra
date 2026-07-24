//! PostgreSQL-backed OperatorRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::OperatorRepository;
use crate::domain::{Operator, OperatorError, OperatorStatus};
use crate::entities::{
    ActiveModel as OperatorActiveModel,
    Column as OperatorColumn,
    Entity as OperatorEntity,
    Model as OperatorModel,
};

/// SeaORM-backed operator repository.
pub struct PostgresOperatorRepository {
    pub db: DatabaseConnection,
}

impl PostgresOperatorRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OperatorRepository for PostgresOperatorRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, operator: &mut Operator) -> Result<(), OperatorError> {
        let model = domain_to_model(operator)?;
        let exists = OperatorEntity::find_by_id(operator.id)
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            OperatorEntity::update(OperatorActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OperatorError::DatabaseError(e.to_string()))?;
        } else {
            OperatorEntity::insert(OperatorActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OperatorError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find()
            .filter(OperatorColumn::TradeLicenseNo.eq(license))
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find()
            .filter(OperatorColumn::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find()
            .filter(OperatorColumn::Subdomain.eq(subdomain))
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn list_by_status(&self, status: Option<&str>) -> Result<Vec<Operator>, OperatorError> {
        let mut query = OperatorEntity::find();
        if let Some(s) = status {
            query = query.filter(OperatorColumn::Status.eq(s));
        }
        let models = query
            .all(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

// ─── Domain ↔ Model conversion ───────────────────────────────────────────────

fn domain_to_model(op: &Operator) -> Result<OperatorModel, OperatorError> {
    Ok(OperatorModel {
        id: op.id,
        legal_name: op.legal_name.clone(),
        trade_license_no: op.trade_license_no.clone(),
        country: op.country.clone(),
        status: op.status.as_str().to_string(),
        subdomain: op.subdomain.clone(),
        email: op.email.clone(),
        verification_token_hash: op.verification_token_hash.clone(),
        provisioned_at: op.provisioned_at,
        created_at: op.created_at,
        updated_at: op.updated_at,
    })
}

fn model_to_domain(m: OperatorModel) -> Result<Operator, OperatorError> {
    let status = OperatorStatus::parse_str(&m.status)
        .ok_or_else(|| OperatorError::NotFound(m.id))?;

    Ok(Operator {
        id: m.id,
        legal_name: m.legal_name,
        trade_license_no: m.trade_license_no,
        country: m.country,
        status,
        subdomain: m.subdomain,
        email: m.email,
        verification_token_hash: m.verification_token_hash,
        provisioned_at: m.provisioned_at,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}
