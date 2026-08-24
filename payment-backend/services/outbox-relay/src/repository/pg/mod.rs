//! PostgreSQL repository for Outbox Relay.
//!
//! Persists [`OutboxEntry`] to the `outbox_entries` table.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::outbox_entry::{self, Entity as OutboxEntryEntity, Column as OutboxEntryColumn};
use crate::repository::OutboxRepository;

#[derive(Clone)]
pub struct PostgresOutboxRepository {
    db: sea_orm::DatabaseConnection,
}

impl PostgresOutboxRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OutboxRepository for PostgresOutboxRepository {
    async fn load_entry(&self, outbox_id: Uuid) -> Result<Option<OutboxEntry>, OutboxRelayError> {
        let result = OutboxEntryEntity::find_by_id(outbox_id)
            .one(&self.db)
            .await
            .map_err(|e| OutboxRelayError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model))),
            None => Ok(None),
        }
    }

    async fn save_entry(&self, entry: &OutboxEntry) -> Result<(), OutboxRelayError> {
        let model = domain_to_model(entry);

        outbox_entry::Entity::insert(model.clone())
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(outbox_entry::Column::EntryId)
                    .update_columns([
                        outbox_entry::Column::Published,
                        outbox_entry::Column::PublishedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| OutboxRelayError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_unpublished(&self, batch_size: u32) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        let results = OutboxEntryEntity::find()
            .filter(OutboxEntryColumn::Published.eq(false))
            .order_by_asc(OutboxEntryColumn::CreatedAt)
            .limit(batch_size as u64)
            .all(&self.db)
            .await
            .map_err(|e| OutboxRelayError::DatabaseError(e.to_string()))?;

        Ok(results.into_iter().map(model_to_domain).collect())
    }

    async fn mark_published(&self, outbox_id: Uuid) -> Result<(), OutboxRelayError> {
        use chrono::Utc;
        use sea_orm::Set;

        let mut model: outbox_entry::ActiveModel = OutboxEntryEntity::find_by_id(outbox_id)
            .one(&self.db)
            .await
            .map_err(|e| OutboxRelayError::PublishFailed(e.to_string()))?
            .ok_or(OutboxRelayError::EntryNotFound(outbox_id))?
            .into();

        model.published = Set(true);
        model.published_at = Set(Some(Utc::now()));

        OutboxEntryEntity::update(model)
            .exec(&self.db)
            .await
            .map_err(|e| OutboxRelayError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn count_unpublished(&self) -> Result<u64, OutboxRelayError> {
        let count = OutboxEntryEntity::find()
            .filter(OutboxEntryColumn::Published.eq(false))
            .count(&self.db)
            .await
            .map_err(|e| OutboxRelayError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    async fn list_entries(&self) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        let results = OutboxEntryEntity::find()
            .order_by_desc(OutboxEntryColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| OutboxRelayError::DatabaseError(e.to_string()))?;

        Ok(results.into_iter().map(model_to_domain).collect())
    }
}

// ─── Domain <-> Entity conversion helpers ─────────────────────────────────────

fn domain_to_model(entry: &OutboxEntry) -> outbox_entry::ActiveModel {
    outbox_entry::ActiveModel {
        entry_id: sea_orm::ActiveValue::Set(entry.outbox_id),
        aggregate_type: sea_orm::ActiveValue::Set(entry.aggregate_type.clone()),
        aggregate_id: sea_orm::ActiveValue::Set(entry.aggregate_id),
        event_type: sea_orm::ActiveValue::Set(entry.event_type.clone()),
        payload: sea_orm::ActiveValue::Set(if entry.payload.is_empty() {
            vec![0u8; 0]
        } else {
            entry.payload.clone()
        }),
        published: sea_orm::ActiveValue::Set(entry.published_at.is_some()),
        created_at: sea_orm::ActiveValue::Set(entry.created_at),
        published_at: sea_orm::ActiveValue::Set(entry.published_at),
    }
}

fn model_to_domain(m: outbox_entry::Model) -> OutboxEntry {
    OutboxEntry {
        outbox_id: m.entry_id,
        aggregate_type: m.aggregate_type,
        aggregate_id: m.aggregate_id,
        event_type: m.event_type,
        event_version: 1, // not stored in entity; default to 1
        payload: m.payload,
        created_at: m.created_at,
        published_at: m.published_at,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_entry() -> OutboxEntry {
        OutboxEntry {
            outbox_id: Uuid::now_v7(),
            aggregate_type: "payment".into(),
            aggregate_id: Uuid::now_v7(),
            event_type: "PaymentAuthorized".into(),
            event_version: 1,
            payload: b"{\"key\":\"value\"}".to_vec(),
            created_at: Utc::now(),
            published_at: None,
        }
    }

    #[tokio::test]
    async fn test_domain_to_model() {
        let entry = sample_entry();
        let model = domain_to_model(&entry);

        assert_eq!(model.entry_id.unwrap(), entry.outbox_id);
        assert_eq!(model.event_type.unwrap(), "PaymentAuthorized");
        assert_eq!(model.published.unwrap(), false);
    }

    #[tokio::test]
    async fn test_model_to_domain() {
        let entry = sample_entry();
        let now = Utc::now();
        let entity = outbox_entry::Model {
            entry_id: entry.outbox_id,
            aggregate_type: "payment".into(),
            aggregate_id: entry.aggregate_id,
            event_type: "PaymentCaptured".into(),
            payload: b"test".to_vec(),
            published: true,
            created_at: now,
            published_at: Some(now),
        };

        let domain = model_to_domain(entity);
        assert_eq!(domain.outbox_id, entry.outbox_id);
        assert_eq!(domain.event_type, "PaymentCaptured");
        assert!(domain.published_at.is_some());
        assert_eq!(domain.event_version, 1);
    }

    #[tokio::test]
    async fn test_unpublished_entry() {
        let entry = sample_entry();
        let model = domain_to_model(&entry);
        assert!(!model.published.unwrap());
        assert!(model.published_at.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_published_entry() {
        let mut entry = sample_entry();
        entry.published_at = Some(Utc::now());
        let model = domain_to_model(&entry);
        assert!(model.published.unwrap());
        assert!(model.published_at.unwrap().is_some());
    }
}
