//! PostgreSQL-backed OnboardingRepository using SeaORM CRUD.
//!
//! NOTE: The SeaORM entity (`onboarding_request.rs`) is missing several domain fields
//! (display_name, health_status, encrypted_credentials, last_tested_at,
//! last_test_result, credential_expires_at). These will be added in Phase 4.
//! For now, they default when loading from the database.
//!
//! Field name mismatch: domain uses `link_id` as PK, entity uses `onboarding_id`.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::onboarding_request::{
    ActiveModel as OnboardingActiveModel, Column as OnboardingColumn,
    Entity as OnboardingEntity, Model as OnboardingModel,
};

/// PostgreSQL-backed repository implementing OnboardingRepository.
#[derive(Clone)]
pub struct PostgresOnboardingRepository {
    pub db: sea_orm::DatabaseConnection,
}

impl PostgresOnboardingRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── Enum parsing helpers (entity uses strings, domain uses Display but not FromStr) ─

fn parse_status(s: &str) -> Option<OnboardingStatus> {
    match s {
        "draft" => Some(OnboardingStatus::Draft),
        "credentials_submitted" => Some(OnboardingStatus::CredentialsSubmitted),
        "testing" => Some(OnboardingStatus::Testing),
        "active" => Some(OnboardingStatus::Active),
        "deactivated" => Some(OnboardingStatus::Deactivated),
        "revoked" => Some(OnboardingStatus::Revoked),
        _ => None,
    }
}

// TODO Phase 4: Add parse_health_status() when entity gains health_status column

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn domain_to_model(request: &OnboardingRequest) -> Result<OnboardingModel, OnboardingError> {
    let credentials_json = serde_json::to_string(&request.credentials)
        .map_err(|e| OnboardingError::InvalidFieldValue(format!("Serialize credentials: {e}")))?;
    let test_result_json = request.last_test_result.as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| OnboardingError::InvalidFieldValue(format!("Serialize test_result: {e}")))?;

    Ok(OnboardingModel {
        onboarding_id: request.link_id,
        operator_id: request.operator_id,
        connector_id: request.connector_id.clone(),
        credentials_json,
        environment: request.environment.clone(),
        status: request.status.to_string(),
        // TODO: Phase 4 - entity needs health_status, display_name, encrypted_credentials columns
        test_result: test_result_json,
        error_message: None,
        created_at: request.created_at,
        updated_at: request.updated_at,
    })
}

fn model_to_domain(m: OnboardingModel) -> Result<OnboardingRequest, OnboardingError> {
    let credentials: std::collections::HashMap<String, String> = serde_json::from_str(&m.credentials_json)
        .map_err(|e| OnboardingError::InvalidFieldValue(format!("Deserialize credentials: {e}")))?;
    let last_test_result = match m.test_result {
        Some(ref json) if !json.is_empty() => {
            Some(serde_json::from_str(json)
                .map_err(|e| OnboardingError::InvalidFieldValue(format!("Deserialize test_result: {e}")))?)
        }
        _ => None,
    };
    let status = parse_status(&m.status)
        .ok_or_else(|| OnboardingError::InvalidFieldValue(format!("Invalid status: {}", m.status)))?;

    Ok(OnboardingRequest {
        link_id: m.onboarding_id,
        operator_id: m.operator_id,
        connector_id: m.connector_id,
        display_name: String::new(),  // TODO: Phase 4 - add display_name column
        environment: m.environment,
        status,
        health_status: HealthStatus::Unknown, // TODO: Phase 4 - add health_status column
        credentials,
        encrypted_credentials: Vec::new(), // TODO: Phase 4 - add encrypted_credentials column
        last_tested_at: None, // TODO: Phase 4 - add last_tested_at column
        last_test_result,
        credential_expires_at: None, // TODO: Phase 4 - add credential_expires_at column
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

// ─── OnboardingRepository Trait Implementation ───────────────────────────────

use crate::repository::OnboardingRepository;

#[async_trait]
impl OnboardingRepository for PostgresOnboardingRepository {
    async fn load(&self, id: Uuid) -> Result<Option<OnboardingRequest>, OnboardingError> {
        let result = OnboardingEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OnboardingError::InvalidFieldValue(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, request: &OnboardingRequest) -> Result<(), OnboardingError> {
        let model = domain_to_model(request)?;

        let exists = OnboardingEntity::find_by_id(request.link_id)
            .one(&self.db)
            .await
            .map_err(|e| OnboardingError::InvalidFieldValue(format!("Database error: {e}")))?
            .is_some();

        if exists {
            OnboardingEntity::update(OnboardingActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OnboardingError::InvalidFieldValue(format!("Database error: {e}")))?;
        } else {
            OnboardingEntity::insert(OnboardingActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OnboardingError::InvalidFieldValue(format!("Database error: {e}")))?;
        }

        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        let results = OnboardingEntity::find()
            .filter(OnboardingColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| OnboardingError::InvalidFieldValue(format!("Database error: {e}")))?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn find_active(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        let results = OnboardingEntity::find()
            .filter(OnboardingColumn::OperatorId.eq(operator_id))
            .filter(OnboardingColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| OnboardingError::InvalidFieldValue(format!("Database error: {e}")))?;

        results.into_iter().map(model_to_domain).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_request() -> OnboardingRequest {
        OnboardingRequest::new(
            Uuid::now_v7(),
            "checkout_com".to_string(),
            "Test Gateway".to_string(),
            "sandbox".to_string(),
        )
    }

    #[test]
    fn test_onboarding_domain_entity_roundtrip() {
        let request = sample_request();

        let model = domain_to_model(&request).unwrap();
        let roundtrip = model_to_domain(model).unwrap();

        // Primary key mapping: onboarding_id ↔ link_id
        assert_eq!(roundtrip.link_id, request.link_id);
        assert_eq!(roundtrip.operator_id, request.operator_id);
        assert_eq!(roundtrip.connector_id, "checkout_com");
        assert_eq!(roundtrip.environment, "sandbox");
        assert_eq!(roundtrip.status, OnboardingStatus::Draft);
        assert_eq!(roundtrip.health_status, HealthStatus::Unknown); // default from entity
        assert_eq!(roundtrip.credentials.len(), 0); // starts empty in new()
        assert!(roundtrip.last_test_result.is_none());
        assert!(roundtrip.encrypted_credentials.is_empty()); // Phase 4 gap
        assert!(roundtrip.last_tested_at.is_none()); // Phase 4 gap
    }

    #[test]
    fn test_status_serialization() {
        assert_eq!(OnboardingStatus::Draft.to_string(), "draft");
        assert_eq!(OnboardingStatus::CredentialsSubmitted.to_string(), "credentials_submitted");
        assert_eq!(OnboardingStatus::Active.to_string(), "active");
        assert_eq!(OnboardingStatus::Revoked.to_string(), "revoked");

        assert_eq!(parse_status("draft"), Some(OnboardingStatus::Draft));
        assert_eq!(parse_status("active"), Some(OnboardingStatus::Active));
        assert_eq!(parse_status("invalid"), None);

        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
    }
}
