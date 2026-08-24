//! PostgreSQL-backed OperatorRepository using SeaORM CRUD.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::operator::{
    ActiveModel as OperatorActiveModel, Column as OperatorColumn,
    Entity as OperatorEntity, Model as OperatorModel,
};

#[derive(Clone)]
pub struct PostgresOperatorRepository {
    pub db: sea_orm::DatabaseConnection,
}

impl PostgresOperatorRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn domain_to_model(op: &Operator) -> OperatorModel {
    OperatorModel {
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
    }
}

fn model_to_domain(m: OperatorModel) -> Result<Operator, OperatorError> {
    let status = OperatorStatus::parse_str(&m.status)
        .ok_or_else(|| OperatorError::InvalidTradeLicenseFormat(format!("Invalid status: {}", m.status)))?;

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

// ─── OperatorRepository Trait Implementation ─────────────────────────────────

use crate::repository::OperatorRepository;

#[async_trait]
impl OperatorRepository for PostgresOperatorRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, operator: &mut Operator) -> Result<(), OperatorError> {
        let model = domain_to_model(operator);

        let exists = OperatorEntity::find_by_id(operator.id)
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?
            .is_some();

        if exists {
            OperatorEntity::update(OperatorActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?;
        } else {
            OperatorEntity::insert(OperatorActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?;
        }

        Ok(())
    }

    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find()
            .filter(OperatorColumn::TradeLicenseNo.eq(license))
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find()
            .filter(OperatorColumn::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Operator>, OperatorError> {
        let result = OperatorEntity::find()
            .filter(OperatorColumn::Subdomain.eq(subdomain))
            .one(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn list_by_status(&self, status: Option<&str>) -> Result<Vec<Operator>, OperatorError> {
        let mut query = OperatorEntity::find();

        if let Some(s) = status {
            query = query.filter(OperatorColumn::Status.eq(s));
        }

        let results = query
            .all(&self.db)
            .await
            .map_err(|e| OperatorError::DatabaseError(format!("{e}")))?;

        results.into_iter().map(model_to_domain).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_operator_domain_entity_roundtrip() {
        let now = Utc::now();
        let op = Operator::new(
            Uuid::now_v7(),
            "Acme Corp".into(),
            "CN-12345".into(),
            "AE".into(),
            "admin@acme.com".into(),
            "acme".into(),
        );

        let model = domain_to_model(&op);
        let roundtrip = model_to_domain(model).unwrap();

        assert_eq!(roundtrip.id, op.id);
        assert_eq!(roundtrip.legal_name, "Acme Corp");
        assert_eq!(roundtrip.trade_license_no, "CN-12345");
        assert_eq!(roundtrip.country, "AE");
        assert_eq!(roundtrip.email, "admin@acme.com");
        assert_eq!(roundtrip.subdomain, "acme");
        assert_eq!(roundtrip.status, OperatorStatus::Pending);
        assert!(roundtrip.verification_token_hash.is_none());
        assert!(roundtrip.provisioned_at.is_none());
    }

    #[test]
    fn test_operator_status_serialization() {
        assert_eq!(OperatorStatus::Pending.as_str(), "pending");
        assert_eq!(OperatorStatus::ActiveVerified.as_str(), "active_verified");
        assert_eq!(OperatorStatus::Suspended.as_str(), "suspended");

        assert_eq!(OperatorStatus::parse_str("pending"), Some(OperatorStatus::Pending));
        assert_eq!(OperatorStatus::parse_str("active_verified"), Some(OperatorStatus::ActiveVerified));
        assert_eq!(OperatorStatus::parse_str("invalid"), None);
    }
}
