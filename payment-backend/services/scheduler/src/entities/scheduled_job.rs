//! ScheduledJob entity — `scheduled_jobs` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduled_jobs")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub job_id: Uuid,
    pub job_type: String,
    pub schedule_expr: String,
    pub payload_json: String,
    pub status: String,
    pub max_retries: i32,
    pub retry_count: i32,
    pub last_run_at: Option<DateTimeUtc>,
    pub next_run_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
