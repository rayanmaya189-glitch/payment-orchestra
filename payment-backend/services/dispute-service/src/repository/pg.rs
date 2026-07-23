//! PostgreSQL-backed DisputeRepository using SeaORM + platform-db entities.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::DisputeRepository;
use crate::domain::*;
use crate::entities::{
    Entity as ChargebackCaseEntity,
    ActiveModel as ChargebackCaseActiveModel,
    Model as ChargebackCaseModel,
    Column as ChargebackCaseColumn,
};

/// SeaORM-backed dispute repository.
pub struct PostgresDisputeRepository {
    pub db: DatabaseConnection,
}

impl PostgresDisputeRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DisputeRepository for PostgresDisputeRepository {
    async fn load(&self, id: Uuid) -> Result<Option<ChargebackCase>, DisputeError> {
        let result = ChargebackCaseEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, case: &ChargebackCase) -> Result<(), DisputeError> {
        let model = domain_to_model(case)?;
        let exists = ChargebackCaseEntity::find_by_id(case.chargeback_id)
            .one(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            ChargebackCaseEntity::update(ChargebackCaseActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| DisputeError::DatabaseError(e.to_string()))?;
        } else {
            ChargebackCaseEntity::insert(ChargebackCaseActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| DisputeError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Vec<ChargebackCase>, DisputeError> {
        let models = ChargebackCaseEntity::find()
            .filter(ChargebackCaseColumn::PaymentIntentId.eq(payment_intent_id))
            .all(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_open_cases(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        // Open = not won, not lost, not accepted
        let models = ChargebackCaseEntity::find()
            .filter(ChargebackCaseColumn::OperatorId.eq(operator_id))
            .filter(
                sea_orm::Condition::all()
                    .add(ChargebackCaseColumn::Outcome.is_null()),
            )
            .all(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let models = ChargebackCaseEntity::find()
            .filter(ChargebackCaseColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| DisputeError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

// ─── Domain ↔ Model conversion ───────────────────────────────────────────────

fn domain_to_model(case: &ChargebackCase) -> Result<ChargebackCaseModel, DisputeError> {
    Ok(ChargebackCaseModel {
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
        outcome: case.outcome.as_ref().map(|o| match o {
            ChargebackOutcome::Won => "won",
            ChargebackOutcome::Lost => "lost",
            ChargebackOutcome::Accepted => "accepted",
            ChargebackOutcome::Escalated => "escalated",
        }.to_string()),
        resolution_note: case.resolution_note.clone(),
        submissions: serde_json::to_value(&case.submissions)
            .map_err(|e| DisputeError::DatabaseError(format!("Serialize submissions: {}", e)))?,
    })
}

fn model_to_domain(m: ChargebackCaseModel) -> Result<ChargebackCase, DisputeError> {
    let status: ChargebackStatus = m.status.parse().map_err(|e: String| DisputeError::DatabaseError(e))?;

    let submissions: Vec<RepresentmentSubmission> = serde_json::from_value(m.submissions)
        .map_err(|e| DisputeError::DatabaseError(format!("Deserialize submissions: {}", e)))?;

    let outcome = m.outcome.as_ref().and_then(|o| match o.as_str() {
        "won" => Some(ChargebackOutcome::Won),
        "lost" => Some(ChargebackOutcome::Lost),
        "accepted" => Some(ChargebackOutcome::Accepted),
        "escalated" => Some(ChargebackOutcome::Escalated),
        _ => None,
    });

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
        outcome,
        resolution_note: m.resolution_note,
        submissions,
    })
}
