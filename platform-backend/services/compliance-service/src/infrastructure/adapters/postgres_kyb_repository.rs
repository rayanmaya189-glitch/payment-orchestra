use async_trait::async_trait;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;
use crate::domain::aggregates::KybCase;
use crate::domain::value_objects::{KybStatus, KybDecision};
use crate::domain::rules::KybCaseRepository;
use crate::infrastructure::entities::kyb_case_entity;
use platform_error::PlatformError;

pub struct PostgresKybRepository { db: DatabaseConnection }
impl PostgresKybRepository {
    pub fn new(db: DatabaseConnection) -> Self { Self { db } }
}

#[async_trait]
impl KybCaseRepository for PostgresKybRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<KybCase>, PlatformError> {
        let m = kyb_case_entity::Entity::find_by_id(id).one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, PlatformError> {
        let m = kyb_case_entity::Entity::find()
            .filter(kyb_case_entity::Column::OperatorId.eq(operator_id))
            .one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn save(&self, case: &KybCase) -> Result<(), PlatformError> {
        let existing = kyb_case_entity::Entity::find_by_id(case.case_id).one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        if let Some(model) = existing {
            let mut a = kyb_case_entity::ActiveModel::from(model);
            a.status = Set(case.status.as_str().to_string());
            a.assigned_compliance_officer = Set(case.assigned_officer);
            a.risk_score = Set(case.risk_score);
            a.decision = Set(case.decision.as_ref().map(|d| d.as_str().to_string()));
            a.decision_reason = Set(case.decision_reason.clone());
            a.reviewed_at = Set(case.reviewed_at.map(|dt| dt.into()));
            a.decided_at = Set(case.decided_at.map(|dt| dt.into()));
            a.update(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        } else {
            let a = kyb_case_entity::ActiveModel {
                id: Set(case.case_id), operator_id: Set(case.operator_id),
                status: Set(case.status.as_str().to_string()),
                assigned_compliance_officer: Set(case.assigned_officer),
                risk_score: Set(case.risk_score),
                decision: Set(case.decision.as_ref().map(|d| d.as_str().to_string())),
                decision_reason: Set(case.decision_reason.clone()),
                notes: Set(case.notes.clone()),
                submitted_at: Set(case.submitted_at.into()),
                reviewed_at: Set(case.reviewed_at.map(|dt| dt.into())),
                decided_at: Set(case.decided_at.map(|dt| dt.into())),
                created_at: Set(case.created_at.into()),
            };
            a.insert(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        }
        Ok(())
    }
    async fn list_pending(&self) -> Result<Vec<KybCase>, PlatformError> {
        let ms = kyb_case_entity::Entity::find()
            .filter(kyb_case_entity::Column::Status.is_in(vec!["submitted", "under_review"]))
            .all(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        Ok(ms.into_iter().map(|m| m.into()).collect())
    }
}

impl From<kyb_case_entity::Model> for KybCase {
    fn from(m: kyb_case_entity::Model) -> Self {
        KybCase {
            case_id: m.id, operator_id: m.operator_id,
            status: KybStatus::from_str(&m.status),
            assigned_officer: m.assigned_compliance_officer,
            risk_score: m.risk_score,
            decision: m.decision.as_deref().map(KybDecision::from_str),
            decision_reason: m.decision_reason, notes: m.notes,
            submitted_at: m.submitted_at.into(),
            reviewed_at: m.reviewed_at.map(|dt| dt.into()),
            decided_at: m.decided_at.map(|dt| dt.into()),
            created_at: m.created_at.into(),
            documents: Vec::new(), uncommitted_events: Vec::new(),
        }
    }
}
