use async_trait::async_trait;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use crate::domain::aggregates::Dispute;
use crate::domain::value_objects::{DisputeStatus, DisputeDecision};
use crate::domain::rules::DisputeRepository;
use crate::infrastructure::entities::dispute_entity;
use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

pub struct PostgresDisputeRepository { db: DatabaseConnection }
impl PostgresDisputeRepository { pub fn new(db: DatabaseConnection) -> Self { Self { db } } }

#[async_trait]
impl DisputeRepository for PostgresDisputeRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Dispute>, PlatformError> {
        let m = dispute_entity::Entity::find_by_id(id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn save(&self, d: &Dispute) -> Result<(), PlatformError> {
        let existing = dispute_entity::Entity::find_by_id(d.dispute_id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        if let Some(model) = existing {
            let mut a = dispute_entity::ActiveModel::from(model);
            a.status = Set(d.status.as_str().to_string());
            a.evidence = Set(d.evidence.as_ref().and_then(|v| serde_json::to_value(v).ok()));
            a.decision = Set(d.decision.as_ref().map(|dec| dec.as_str().to_string()));
            a.decision_reason = Set(d.decision_reason.clone());
            a.resolved_at = Set(d.resolved_at.map(|dt| dt.into()));
            a.updated_at = Set(d.updated_at.into());
            a.update(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        } else {
            let a = dispute_entity::ActiveModel {
                dispute_id: Set(d.dispute_id), payment_intent_id: Set(d.payment_intent_id),
                operator_id: Set(d.operator_id), status: Set(d.status.as_str().to_string()),
                reason: Set(d.reason.clone()), reason_code: Set(d.reason_code.clone()),
                disputed_amount_minor_units: Set(d.disputed_amount.amount_minor_units),
                currency: Set(d.disputed_amount.currency.0.clone()),
                acquirer_reference: Set(d.acquirer_reference.clone()),
                connector_id: Set(d.connector_id.clone()),
                acquirer_dispute_id: Set(d.acquirer_dispute_id.clone()),
                evidence: Set(d.evidence.as_ref().and_then(|v| serde_json::to_value(v).ok())),
                decision: Set(d.decision.as_ref().map(|dec| dec.as_str().to_string())),
                decision_reason: Set(d.decision_reason.clone()),
                opened_at: Set(d.opened_at.into()), respond_by: Set(d.respond_by.map(|dt| dt.into())),
                resolved_at: Set(d.resolved_at.map(|dt| dt.into())),
                created_at: Set(d.created_at.into()), updated_at: Set(d.updated_at.into()),
            };
            a.insert(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        }
        Ok(())
    }
}

impl From<dispute_entity::Model> for Dispute {
    fn from(m: dispute_entity::Model) -> Self {
        Dispute {
            dispute_id: m.dispute_id, payment_intent_id: m.payment_intent_id, operator_id: m.operator_id,
            status: DisputeStatus::Opened, reason: m.reason, reason_code: m.reason_code,
            disputed_amount: Money { amount_minor_units: m.disputed_amount_minor_units, currency: CurrencyCode::new(&m.currency).unwrap() },
            acquirer_reference: m.acquirer_reference, connector_id: m.connector_id,
            acquirer_dispute_id: m.acquirer_dispute_id,
            evidence: m.evidence.and_then(|v| serde_json::from_value(v.into()).ok()),
            decision: None,
            decision_reason: m.decision_reason,
            opened_at: m.opened_at.into(), respond_by: m.respond_by.map(|dt| dt.into()),
            resolved_at: m.resolved_at.map(|dt| dt.into()),
            created_at: m.created_at.into(), updated_at: m.updated_at.into(),
        }
    }
}
