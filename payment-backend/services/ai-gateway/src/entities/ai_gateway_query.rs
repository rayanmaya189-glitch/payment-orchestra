//! AiGatewayQuery entity — `ai_gateway_queries` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_gateway_queries")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub query_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub model: String,
    pub prompt: String,
    pub response: Option<String>,
    pub status: String,
    pub latency_ms: i32,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
