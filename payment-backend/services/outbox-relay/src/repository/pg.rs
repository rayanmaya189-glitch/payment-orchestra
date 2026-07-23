//! PostgreSQL-backed OutboxRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::OutboxRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as OutboxActiveModel,
    Column as OutboxColumn,
    Entity as OutboxEntity,
    Model as OutboxModel,
};

pub struct PostgresOutboxRepository {
    pub db: DatabaseConnection,
}

impl PostgresOutboxRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OutboxRepository for PostgresOutboxRepository {
    async fn find_unpublished(&self, batch_size: u32) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        let entries = OutboxEntity::find()
            .filter(OutboxColumn::Published.eq(false))
            .limit(batch_size as u64)
            .all(&self.db)
            .await
            .map_err(|e| OutboxRelayError::EntryNotFound(Uuid::default()))?;
        entries.into_iter().map(model_to_domain).collect()
    }

    async fn mark_published(&self, entry_id: Uuid) -> Result<(), OutboxRelayError> {
        let mut model: OutboxActiveModel = OutboxEntity::find_by_id(entry_id)
            .one(&self.db)
            .await
            .map_err(|e| OutboxRelayError::EntryNotFound(entry_id))?
            .ok_or_else(|| OutboxRelayError::EntryNotFound(entry_id))?
            .into();

        model.published = Set(true);
        model.published_at = Set(Some(Utc::now()));

        OutboxEntity::update(model)
            .exec(&self.db)
            .await
            .map_err(|e| OutboxRelayError::EntryNotFound(entry_id))?;
        Ok(())
    }

    async fn count_unpublished(&self) -> Result<u64, OutboxRelayError> {
        let count = OutboxEntity::find()
            .filter(OutboxColumn::Published.eq(false))
            .count(&self.db)
            .await
            .map_err(|e| OutboxRelayError::EntryNotFound(Uuid::default()))?;
        Ok(count)
    }
}

fn model_to_domain(m: OutboxModel) -> Result<OutboxEntry, OutboxRelayError> {
    Ok(OutboxEntry {
        entry_id: m.entry_id,
        aggregate_type: m.aggregate_type,
        aggregate_id: m.aggregate_id,
        event_type: m.event_type,
        payload: m.payload,
        published: m.published,
        created_at: m.created_at,
        published_at: m.published_at,
    })
}
