//! PostgreSQL-backed OnboardingRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::OnboardingRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as OnboardingActiveModel,
    Column as OnboardingColumn,
    Entity as OnboardingEntity,
    Model as OnboardingModel,
};

pub struct PostgresOnboardingRepository {
    pub db: DatabaseConnection,
}

impl PostgresOnboardingRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OnboardingRepository for PostgresOnboardingRepository {
    async fn load(&self, id: Uuid) -> Result<Option<OnboardingRequest>, OnboardingError> {
        let result = OnboardingEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OnboardingError::NotFound(id))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, request: &OnboardingRequest) -> Result<(), OnboardingError> {
        let model = domain_to_model(request);
        let exists = OnboardingEntity::find_by_id(request.onboarding_id)
            .one(&self.db)
            .await
            .map_err(|e| OnboardingError::NotFound(request.onboarding_id))?
            .is_some();

        if exists {
            OnboardingEntity::update(OnboardingActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OnboardingError::NotFound(request.onboarding_id))?;
        } else {
            OnboardingEntity::insert(OnboardingActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OnboardingError::NotFound(request.onboarding_id))?;
        }
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        let models = OnboardingEntity::find()
            .filter(OnboardingColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| OnboardingError::NotFound(operator_id))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_active_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        let models = OnboardingEntity::find()
            .filter(OnboardingColumn::OperatorId.eq(operator_id))
            .filter(OnboardingColumn::Status.is_in(vec!["pending", "testing"]))
            .all(&self.db)
            .await
            .map_err(|e| OnboardingError::NotFound(operator_id))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        let models = OnboardingEntity::find()
            .filter(OnboardingColumn::ConnectorId.eq(connector_id))
            .all(&self.db)
            .await
            .map_err(|e| OnboardingError::ConnectorNotFound(connector_id.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

fn domain_to_model(r: &OnboardingRequest) -> OnboardingModel {
    OnboardingModel {
        onboarding_id: r.onboarding_id,
        operator_id: r.operator_id,
        connector_id: r.connector_id.clone(),
        credentials_json: r.credentials_json.clone(),
        environment: r.environment.clone(),
        status: r.status.clone(),
        test_result: r.test_result.clone(),
        error_message: r.error_message.clone(),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn model_to_domain(m: OnboardingModel) -> Result<OnboardingRequest, OnboardingError> {
    Ok(OnboardingRequest {
        onboarding_id: m.onboarding_id,
        operator_id: m.operator_id,
        connector_id: m.connector_id,
        credentials_json: m.credentials_json,
        environment: m.environment,
        status: m.status,
        test_result: m.test_result,
        error_message: m.error_message,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}
