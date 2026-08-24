//! PostgreSQL-backed KybCase repository using SeaORM CRUD.
//!
//! Converts between the domain KybCase model (with KybStatus enum) and
//! the flat SeaORM entity model (kyb_cases table).

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::{ComplianceError, KybCase, KybStatus};
use crate::entities::kyb_case::{
    ActiveModel as KybCaseActiveModel, Column as KybCaseColumn,
    Entity as KybCaseEntity, Model as KybCaseModel,
};
use super::PostgresComplianceRepository;


impl PostgresComplianceRepository {
    /// Load KybCase from DB and convert to domain model.
    pub(super) async fn load_kyb_case_domain(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        let result = KybCaseEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(kyb_case_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    /// Save domain KybCase to DB.
    pub(super) async fn save_kyb_case_domain(&self, kase: &KybCase) -> Result<(), ComplianceError> {
        let model = kyb_case_domain_to_model(kase)?;

        let exists = KybCaseEntity::find_by_id(kase.kyb_case_id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?
            .is_some();

        if exists {
            KybCaseEntity::update(KybCaseActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;
        } else {
            KybCaseEntity::insert(KybCaseActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;
        }
        Ok(())
    }
}

// ─── KybCase implementation block ─────────────────────────────────────────

impl PostgresComplianceRepository {
    /// List all non-terminal KYB cases ordered by submission date.
    pub(super) async fn list_pending_kyb_cases_domain(&self) -> Result<Vec<KybCase>, ComplianceError> {
        let models = KybCaseEntity::find()
            .filter(KybCaseColumn::Status.ne("approved"))
            .filter(KybCaseColumn::Status.ne("rejected"))
            .order_by_asc(KybCaseColumn::SubmittedAt)
            .all(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;

        models.into_iter().map(kyb_case_model_to_domain).collect()
    }

    /// Find a KYB case by operator ID.
    pub(super) async fn find_kyb_by_operator_domain(&self, operator_id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        let result = KybCaseEntity::find()
            .filter(KybCaseColumn::OperatorId.eq(operator_id))
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(kyb_case_model_to_domain(model)?)),
            None => Ok(None),
        }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn kyb_case_domain_to_model(kase: &KybCase) -> Result<KybCaseModel, ComplianceError> {
    let document_ids_json = serde_json::to_value(&kase.document_ids)
        .map_err(|e| ComplianceError::InvalidRequest(format!("Serialize document_ids: {}", e)))?;

    Ok(KybCaseModel {
        kyb_case_id: kase.kyb_case_id,
        operator_id: kase.operator_id,
        status: kase.status.as_str().to_string(),
        submitted_by: kase.submitted_by,
        document_ids: document_ids_json,
        ocr_extracted_fields: kase.ocr_extracted_fields.clone(),
        partner_decision: kase.partner_decision.clone(),
        rejection_reason: kase.rejection_reason.clone(),
        submitted_at: kase.submitted_at,
        resolved_at: kase.resolved_at,
        updated_at: kase.updated_at,
    })
}

fn kyb_case_model_to_domain(m: KybCaseModel) -> Result<KybCase, ComplianceError> {
    let document_ids: Vec<Uuid> = serde_json::from_value(m.document_ids)
        .map_err(|e| ComplianceError::InvalidRequest(format!("Deserialize document_ids: {}", e)))?;

    Ok(KybCase {
        kyb_case_id: m.kyb_case_id,
        operator_id: m.operator_id,
        status: KybStatus::parse_str(&m.status).unwrap_or(KybStatus::Submitted),
        submitted_by: m.submitted_by,
        document_ids,
        ocr_extracted_fields: m.ocr_extracted_fields,
        partner_decision: m.partner_decision,
        rejection_reason: m.rejection_reason,
        submitted_at: m.submitted_at,
        resolved_at: m.resolved_at,
        updated_at: m.updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_kyb_case_domain_entity_roundtrip() {
        let now = Utc::now();
        let kase = KybCase {
            kyb_case_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            status: KybStatus::Approved,
            submitted_by: Uuid::now_v7(),
            document_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
            ocr_extracted_fields: None,
            partner_decision: Some("verified".into()),
            rejection_reason: None,
            submitted_at: now,
            resolved_at: Some(now),
            updated_at: now,
        };

        let model = kyb_case_domain_to_model(&kase).unwrap();
        let roundtrip = kyb_case_model_to_domain(model).unwrap();

        assert_eq!(roundtrip.kyb_case_id, kase.kyb_case_id);
        assert_eq!(roundtrip.status, kase.status);
        assert_eq!(roundtrip.document_ids.len(), 2);
        assert_eq!(roundtrip.operator_id, kase.operator_id);
        assert_eq!(roundtrip.partner_decision, kase.partner_decision);
        assert!(roundtrip.resolved_at.is_some());
    }
}
