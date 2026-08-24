//! SagaExecution entity — `saga_executions` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "saga_executions")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub saga_id: Uuid,
    pub saga_type: String,
    #[sea_orm(column_type = "Uuid")]
    pub aggregate_id: Uuid,
    pub status: String,
    pub steps: Json,
    pub current_step: i32,
    pub compensation_running: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub completed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
