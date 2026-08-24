//! Operator entity — `operators` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `operators` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "operators")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub status: String,
    pub subdomain: String,
    pub email: String,
    pub verification_token_hash: Option<String>,
    pub provisioned_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
