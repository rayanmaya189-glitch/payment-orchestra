//! RoutingPolicy entity — `routing_policies` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `routing_policies` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "routing_policies")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub routing_policy_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub name: String,
    pub status: String,
    /// JSONB: serialized Vec<RoutingRule>
    pub rules: Json,
    pub created_at: DateTimeUtc,
    pub activated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
