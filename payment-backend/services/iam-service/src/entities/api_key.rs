//! ApiKey entity — `api_keys` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `api_keys` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub api_key_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub principal_id: Uuid,
    pub name: String,
    pub key_hash: Vec<u8>,
    pub scopes: Json,
    pub status: String,
    pub created_at: DateTimeUtc,
    pub expires_at: Option<DateTimeUtc>,
    pub last_used_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
