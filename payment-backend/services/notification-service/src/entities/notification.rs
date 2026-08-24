//! Notification entity — `notifications` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub notification_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub channel: String,
    pub recipient: String,
    pub template_id: String,
    pub payload_json: String,
    pub subject: Option<String>,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: DateTimeUtc,
    pub sent_at: Option<DateTimeUtc>,
    pub last_error: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
