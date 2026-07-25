//! PostgreSQL-backed Dispute repository using SeaORM CRUD.
//!
//! Converts between the domain ChargebackCase model (with ChargebackStatus enum,
//! Vec<RepresentmentSubmission> JSONB) and the flat SeaORM entity model.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::{
    ActiveModel as ChargebackActiveModel, Column as ChargebackColumn,
    Entity as ChargebackEntity, Model as ChargebackModel,
};

/// PostgreSQL-backed repository implementing DisputeRepository.
#[derive(Clone)]
pub struct PostgresDisputeRepository {
    pub db: sea_orm::DatabaseConnection,
}

impl PostgresDisputeRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── Helper: ChargebackOutcome serialization ─────────────────────────────────

fn outcome_to_string(o: &Option<ChargebackOutcome>) -> Option<String> {
    Some(match o.as_ref()? {
        ChargebackOutcome::Won => "Won",
        ChargebackOutcome::Lost => "Lost",
        ChargebackOutcome::Accepted => "Accepted",
        ChargebackOutcome::Escalated => "Escalated",
    }.to_string())
}

fn string_to_outcome(s: Option<String>) -> Option<ChargebackOutcome> {
    let val = s.as_deref()?;
    match val {
        "Won" => Some(ChargebackOutcome::Won),
        "Lost" => Some(ChargebackOutcome::Lost),
        "Accepted" => Some(ChargebackOutcome::Accepted),
        "Escalated" => Some(ChargebackOutcome::Escalated),
        _ => None,
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn case_domain_to_model(case: &ChargebackCase) -> Result<ChargebackModel, DisputeError> {
    let submissions_json = serde_json::to_value(&case.submissions)
        .map_err(|e| DisputeError::DatabaseError(format!("Serialize submissions: {e}")))?;

    Ok(ChargebackModel {
        chargeback_id: case.chargeback_id,
        operator_id: case.operator_id,
        payment_intent_id: case.payment_intent_id,
        acquirer_link_id: case.acquirer_link_id,
        status: case.status.to_string(),
        reason_code: case.reason_code.clone(),
        amount_minor_units: case.amount_minor_units,
        currency: case.currency.clone(),
        received_at: case.received_at,
        representment_deadline: case.representment_deadline,
        resolved_at: case.resolved_at,
        outcome: outcome_to_string(&case.outcome),
        resolution_note: case.resolution_note.clone(),
        submissions: submissions_json,
    })
}

fn case_model_to_domain(m: ChargebackModel) -> Result<ChargebackCase, DisputeError> {
    let submissions: Vec<RepresentmentSubmission> = serde_json::from_value(m.submissions)
        .map_err(|e| DisputeError::DatabaseError(format!("Deserialize submissions: {e}")))?;
    let status: ChargebackStatus = m.status
        .parse()
        .map_err(|e: String| DisputeError::DatabaseError(format!("Parse status: {e}")))?;

    Ok(ChargebackCase {
        chargeback_id: m.chargeback_id,
        operator_id: m.operator_id,
        payment_intent_id: m.payment_intent_id,
        acquirer_link_id: m.acquirer_link_id,
        status,
        reason_code: m.reason_code,
        amount_minor_units: m.amount_minor_units,
        currency: m.currency,
        received_at: m.received_at,
        representment_deadline: m.representment_deadline,
        resolved_at: m.resolved_at,
        outcome: string_to_outcome(m.outcome),
        resolution_note: m.resolution_note,
        submissions,
    })
}

// ─── DisputeRepository Trait Implementation ──────────────────────────────────

use crate::repository::DisputeRepository;

#[async_trait]
impl DisputeRepository for PostgresDisputeRepository {
    async fn load(&self, id: Uuid) -> Result<Option<ChargebackCase>, DisputeError> {
        let result = ChargebackEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(case_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, case: &ChargebackCase) -> Result<(), DisputeError> {
        let model = case_domain_to_model(case)?;

        let exists = ChargebackEntity::find_by_id(case.chargeback_id)
            .one(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(format!("Database error: {e}")))?
            .is_some();

        if exists {
            ChargebackEntity::update(ChargebackActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| DisputeError::DatabaseError(format!("Database error: {e}")))?;
        } else {
            ChargebackEntity::insert(ChargebackActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| DisputeError::DatabaseError(format!("Database error: {e}")))?;
        }

        Ok(())
    }

    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let results = ChargebackEntity::find()
            .filter(ChargebackColumn::PaymentIntentId.eq(payment_intent_id))
            .all(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(case_model_to_domain)
            .collect()
    }

    async fn find_open_cases(&self, _operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        // Open = not in a resolved status
        let results = ChargebackEntity::find()
            .filter(ChargebackColumn::Status.ne("won"))
            .filter(ChargebackColumn::Status.ne("lost"))
            .filter(ChargebackColumn::Status.ne("accepted"))
            .all(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(case_model_to_domain)
            .collect()
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let results = ChargebackEntity::find()
            .filter(ChargebackColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(case_model_to_domain)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_case() -> ChargebackCase {
        ChargebackCase::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "fraud".into(),
            5000,
            "USD".into(),
        )
        .unwrap()
    }

    #[test]
    fn test_case_domain_entity_roundtrip() {
        let case = sample_case();

        let model = case_domain_to_model(&case).unwrap();
        let roundtrip = case_model_to_domain(model).unwrap();

        assert_eq!(roundtrip.chargeback_id, case.chargeback_id);
        assert_eq!(roundtrip.status, ChargebackStatus::Received);
        assert_eq!(roundtrip.reason_code, "fraud");
        assert_eq!(roundtrip.amount_minor_units, 5000);
        assert_eq!(roundtrip.currency, "USD");
        assert!(roundtrip.submissions.is_empty());
        assert!(roundtrip.resolved_at.is_none());
        assert!(roundtrip.outcome.is_none());
    }

    #[test]
    fn test_outcome_serialization_roundtrip() {
        assert_eq!(string_to_outcome(Some("Won".into())), Some(ChargebackOutcome::Won));
        assert_eq!(string_to_outcome(Some("Lost".into())), Some(ChargebackOutcome::Lost));
        assert_eq!(string_to_outcome(Some("Accepted".into())), Some(ChargebackOutcome::Accepted));
        assert_eq!(string_to_outcome(Some("Escalated".into())), Some(ChargebackOutcome::Escalated));
        assert_eq!(string_to_outcome(None), None);

        let outcome = Some(ChargebackOutcome::Won);
        assert_eq!(outcome_to_string(&outcome), Some("Won".into()));

        assert_eq!(outcome_to_string(&None), None);
    }

    #[test]
    fn test_chargeback_status_serialization() {
        assert_eq!(ChargebackStatus::Received.to_string(), "received");
        assert_eq!(ChargebackStatus::UnderReview.to_string(), "under_review");
        assert_eq!(ChargebackStatus::RepresentmentSubmitted.to_string(), "representment_submitted");
        assert_eq!(ChargebackStatus::Won.to_string(), "won");
        assert_eq!(ChargebackStatus::Lost.to_string(), "lost");
        assert_eq!(ChargebackStatus::Accepted.to_string(), "accepted");
        assert_eq!(ChargebackStatus::Escalated.to_string(), "escalated");

        assert_eq!("received".parse::<ChargebackStatus>().unwrap(), ChargebackStatus::Received);
        assert_eq!("won".parse::<ChargebackStatus>().unwrap(), ChargebackStatus::Won);
    }
}
