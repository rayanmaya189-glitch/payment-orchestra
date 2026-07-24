//! SeaORM entity for the `outbox_entries` table.
//! Written within the same transaction as the event store append
//! to guarantee at-least-once delivery (DB-005/006).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "outbox_entries")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub entry_id: Uuid,
    pub aggregate_type: String,
    #[sea_orm(column_type = "Uuid")]
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub subject: String,
    pub payload: Vec<u8>,
    pub published: bool,
    pub created_at: DateTimeUtc,
    pub published_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
