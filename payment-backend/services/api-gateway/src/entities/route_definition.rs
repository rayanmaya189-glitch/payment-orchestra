//! RouteDefinition entity — `route_definitions` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "route_definitions")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub route_id: Uuid,
    pub path: String,
    pub method: String,
    pub grpc_service: String,
    pub grpc_method: String,
    pub auth_required: bool,
    pub rate_limit_per_second: i32,
    pub rate_limit_burst: i32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
