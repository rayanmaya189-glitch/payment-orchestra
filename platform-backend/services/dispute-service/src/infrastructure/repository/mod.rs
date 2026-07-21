use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::domain::entities::Evidence;
use crate::domain::rules::EvidenceRepository;
use platform_error::PlatformError;

use super::entities::evidence_entity;

pub struct PostgresEvidenceRepository {
    db: DatabaseConnection,
}

impl PostgresEvidenceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl EvidenceRepository for PostgresEvidenceRepository {
    async fn add_evidence(&self, _dispute_id: Uuid, evidence: &Evidence) -> Result<(), PlatformError> {
        let active = evidence_entity::ActiveModel {
            evidence_id: Set(evidence.evidence_id),
            dispute_id: Set(evidence.dispute_id),
            evidence_type: Set(evidence.evidence_type.as_str().to_string()),
            description: Set(evidence.description.clone()),
            file_uri: Set(evidence.file_uri.clone()),
            submitted_by: Set(evidence.submitted_by),
            created_at: Set(evidence.created_at.into()),
        };
        active
            .insert(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        Ok(())
    }

    async fn get_evidence(&self, dispute_id: Uuid) -> Result<Vec<Evidence>, PlatformError> {
        let models = evidence_entity::Entity::find()
            .filter(evidence_entity::Column::DisputeId.eq(dispute_id))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;

        Ok(models.into_iter().map(Evidence::from).collect())
    }
}
