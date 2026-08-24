//! SeaORM entity for the `aggregate_snapshots` table.
//! Composite primary key: (aggregate_type, aggregate_id, as_of_sequence)

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "aggregate_snapshots")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub aggregate_type: String,
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub aggregate_id: Uuid,
    #[sea_orm(primary_key)]
    pub as_of_sequence: i64,
    pub state: Vec<u8>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
