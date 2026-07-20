use async_trait::async_trait;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use crate::domain::aggregates::SettlementBatch;
use crate::domain::value_objects::SettlementStatus;
use crate::domain::rules::SettlementBatchRepository;
use crate::infrastructure::entities::settlement_batch_entity;
use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

pub struct PostgresSettlementRepository { db: DatabaseConnection }
impl PostgresSettlementRepository { pub fn new(db: DatabaseConnection) -> Self { Self { db } } }

#[async_trait]
impl SettlementBatchRepository for PostgresSettlementRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SettlementBatch>, PlatformError> {
        let m = settlement_batch_entity::Entity::find_by_id(id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn save(&self, b: &SettlementBatch) -> Result<(), PlatformError> {
        let existing = settlement_batch_entity::Entity::find_by_id(b.batch_id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        if let Some(model) = existing {
            let mut a = settlement_batch_entity::ActiveModel::from(model);
            a.status = Set(b.status.as_str().to_string());
            a.matched_count = Set(b.matched_count);
            a.unmatched_count = Set(b.unmatched_count);
            a.updated_at = Set(b.updated_at.into());
            a.update(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        } else {
            let a = settlement_batch_entity::ActiveModel {
                batch_id: Set(b.batch_id), operator_id: Set(b.operator_id),
                connector_id: Set(b.connector_id.clone()), status: Set(b.status.as_str().to_string()),
                total_amount_minor_units: Set(b.total_amount.amount_minor_units),
                currency: Set(b.total_amount.currency.0.clone()),
                total_records: Set(b.total_records), matched_count: Set(b.matched_count),
                unmatched_count: Set(b.unmatched_count), exception_count: Set(b.exception_count),
                period_start: Set(b.period_start.clone()), period_end: Set(b.period_end.clone()),
                exceptions: Set(b.exceptions.as_ref().and_then(|v| serde_json::to_value(v).ok())),
                polled_at: Set(b.polled_at.map(|dt| dt.into())),
                matched_at: Set(b.matched_at.map(|dt| dt.into())),
                settled_at: Set(b.settled_at.map(|dt| dt.into())),
                created_at: Set(b.created_at.into()), updated_at: Set(b.updated_at.into()),
            };
            a.insert(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        }
        Ok(())
    }
}

impl From<settlement_batch_entity::Model> for SettlementBatch {
    fn from(m: settlement_batch_entity::Model) -> Self {
        SettlementBatch {
            batch_id: m.batch_id, operator_id: m.operator_id, connector_id: m.connector_id,
            status: SettlementStatus::Pending, total_amount: Money { amount_minor_units: m.total_amount_minor_units, currency: CurrencyCode::new(&m.currency).unwrap() },
            total_records: m.total_records, matched_count: m.matched_count, unmatched_count: m.unmatched_count, exception_count: m.exception_count,
            period_start: m.period_start, period_end: m.period_end,
            exceptions: m.exceptions.and_then(|v| serde_json::from_value(v.into()).ok()),
            polled_at: m.polled_at.map(|dt| dt.into()), matched_at: m.matched_at.map(|dt| dt.into()),
            settled_at: m.settled_at.map(|dt| dt.into()),
            created_at: m.created_at.into(), updated_at: m.updated_at.into(),
        }
    }
}
