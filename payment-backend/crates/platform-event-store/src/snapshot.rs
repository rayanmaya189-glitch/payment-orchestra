//! Aggregate snapshots — periodic materialized state for long event streams.
//! Composite primary key: (aggregate_type, aggregate_id, as_of_sequence).
//! Allows rebuilding aggregate state without replaying the full event stream.

use sea_orm::{DatabaseConnection, EntityTrait, Set, QueryFilter, ColumnTrait, QueryOrder, DbErr};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::snapshot_entity::{self, Entity as SnapshotEntity, ActiveModel as SnapshotActiveModel, Model as SnapshotModel};

/// A materialized snapshot of aggregate state at a given event sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSnapshot {
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub as_of_sequence: i64,
    pub state: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

/// Snapshot store backed by PostgreSQL via SeaORM.
pub struct SnapshotStore {
    db: DatabaseConnection,
}

impl SnapshotStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Save a new snapshot.
    pub async fn save(&self, snapshot: &AggregateSnapshot) -> Result<(), DbErr> {
        let active = SnapshotActiveModel {
            aggregate_type: Set(snapshot.aggregate_type.clone()),
            aggregate_id: Set(snapshot.aggregate_id),
            as_of_sequence: Set(snapshot.as_of_sequence),
            state: Set(snapshot.state.clone()),
            created_at: Set(snapshot.created_at),
        };
        SnapshotEntity::insert(active).exec(&self.db).await?;
        Ok(())
    }

    /// Load the latest snapshot for a given aggregate.
    pub async fn load_latest(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<Option<AggregateSnapshot>, DbErr> {
        let model = SnapshotEntity::find()
            .filter(snapshot_entity::Column::AggregateType.eq(aggregate_type))
            .filter(snapshot_entity::Column::AggregateId.eq(aggregate_id))
            .order_by_desc(snapshot_entity::Column::AsOfSequence)
            .one(&self.db)
            .await?;

        Ok(model.map(model_to_snapshot))
    }

    /// Load a snapshot at or before a given sequence number.
    pub async fn load_at_or_before(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
        max_sequence: i64,
    ) -> Result<Option<AggregateSnapshot>, DbErr> {
        let model = SnapshotEntity::find()
            .filter(snapshot_entity::Column::AggregateType.eq(aggregate_type))
            .filter(snapshot_entity::Column::AggregateId.eq(aggregate_id))
            .filter(snapshot_entity::Column::AsOfSequence.lte(max_sequence))
            .order_by_desc(snapshot_entity::Column::AsOfSequence)
            .one(&self.db)
            .await?;

        Ok(model.map(model_to_snapshot))
    }

    /// Delete all snapshots for a given aggregate (e.g., before re-snapshotting).
    pub async fn delete_all(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<u64, DbErr> {
        let result = SnapshotEntity::delete_many()
            .filter(snapshot_entity::Column::AggregateType.eq(aggregate_type))
            .filter(snapshot_entity::Column::AggregateId.eq(aggregate_id))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }
}

fn model_to_snapshot(m: SnapshotModel) -> AggregateSnapshot {
    AggregateSnapshot {
        aggregate_type: m.aggregate_type,
        aggregate_id: m.aggregate_id,
        as_of_sequence: m.as_of_sequence,
        state: m.state,
        created_at: m.created_at.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let snap = AggregateSnapshot {
            aggregate_type: "PaymentIntent".into(),
            aggregate_id: Uuid::now_v7(),
            as_of_sequence: 42,
            state: vec![1, 2, 3, 4],
            created_at: Utc::now(),
        };

        assert_eq!(snap.aggregate_type, "PaymentIntent");
        assert_eq!(snap.as_of_sequence, 42);
        assert_eq!(snap.state.len(), 4);
    }
}
