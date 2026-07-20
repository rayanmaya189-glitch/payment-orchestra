use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, QueryOrder};
use uuid::Uuid;

use crate::infrastructure::entities::event_store;
use crate::infrastructure::repository::EventStoreRepository;
use platform_error::PlatformError;

pub struct PostgresEventStoreRepository {
    db: DatabaseConnection,
}

impl PostgresEventStoreRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl EventStoreRepository for PostgresEventStoreRepository {
    /// Append event with optimistic concurrency control.
    /// Uses a raw SQL check to enforce event_sequence monotonicity.
    async fn append(
        &self,
        event: &shared_types::events::EventEnvelope,
        expected_sequence: i64,
    ) -> Result<i64, PlatformError> {
        use sea_orm::{ConnectionTrait, Statement};

        let next_sequence = expected_sequence + 1;

        // Optimistic concurrency: only insert if no higher sequence exists
        let result = self.db
            .execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "INSERT INTO event_store (event_id, aggregate_type, aggregate_id, event_sequence, event_type, event_version, payload, occurred_at, actor_type, actor_id, causation_id, correlation_id, trace_context, signature)
                 SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
                 WHERE NOT EXISTS (
                     SELECT 1 FROM event_store
                     WHERE aggregate_type = $2 AND aggregate_id = $3 AND event_sequence >= $4
                 )",
                vec![
                    event.event_id.into(),
                    event.aggregate_type.clone().into(),
                    event.aggregate_id.into(),
                    next_sequence.into(),
                    event.event_type.clone().into(),
                    (event.event_version as i32).into(),
                    serde_json::to_value(&event.payload)
                        .unwrap_or_default()
                        .into(),
                    event.occurred_at.into(),
                    event.actor_type.clone().into(),
                    event.actor_id.into(),
                    event.causation_id.into(),
                    event.correlation_id.into(),
                    event.trace_context.clone().into(),
                    event.signature.clone().into(),
                ],
            ))
            .await
            .map_err(|e| PlatformError::Internal(format!("Event store append failed: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(PlatformError::Conflict(
                platform_error::ConflictError::ConcurrencyViolation
            ));
        }

        Ok(next_sequence)
    }

    async fn load_events(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<Vec<shared_types::events::EventEnvelope>, PlatformError> {
        let models = event_store::Entity::find()
            .filter(event_store::Column::AggregateType.eq(aggregate_type))
            .filter(event_store::Column::AggregateId.eq(aggregate_id))
            .order_by_asc(event_store::Column::EventSequence)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Event store load failed: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_event_envelope()).collect())
    }

    async fn current_sequence(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<i64, PlatformError> {
        use sea_orm::{ConnectionTrait, Statement};

        let result = self.db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COALESCE(MAX(event_sequence), 0) as seq FROM event_store WHERE aggregate_type = $1 AND aggregate_id = $2",
                vec![aggregate_type.into(), aggregate_id.into()],
            ))
            .await
            .map_err(|e| PlatformError::Internal(format!("Event store sequence query failed: {e}")))?;

        match result {
            Some(row) => {
                let seq: i64 = row.try_get("", "seq").unwrap_or(0);
                Ok(seq)
            }
            None => Ok(0),
        }
    }
}
