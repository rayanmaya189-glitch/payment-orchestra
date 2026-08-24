//! MerchantAcquirerLink entity — `merchant_acquirer_links` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "merchant_acquirer_links")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub link_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub connector_id: String,
    pub display_name: String,
    pub environment: String,
    pub encrypted_credentials: Vec<u8>,
    pub credentials_hash: String,
    pub status: String,
    pub health_status: String,
    pub last_tested_at: Option<DateTimeUtc>,
    pub last_healthy_at: Option<DateTimeUtc>,
    pub credentials_expires_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
