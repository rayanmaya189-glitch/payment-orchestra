//! SeaORM entity for the `event_store` table.
//! Composite primary key: (aggregate_type, aggregate_id, event_sequence)

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "event_store")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub aggregate_type: String,
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub aggregate_id: Uuid,
    #[sea_orm(primary_key)]
    pub event_sequence: i64,
    #[sea_orm(column_type = "Uuid", unique)]
    pub event_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub occurred_at: DateTimeUtc,
    pub actor_type: String,
    #[sea_orm(column_type = "Uuid")]
    pub actor_id: Option<Uuid>,
    #[sea_orm(column_type = "Uuid")]
    pub causation_id: Option<Uuid>,
    #[sea_orm(column_type = "Uuid")]
    pub correlation_id: Uuid,
    pub payload: Vec<u8>,
    pub checksum: Option<Vec<u8>>,
    pub encrypted: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
