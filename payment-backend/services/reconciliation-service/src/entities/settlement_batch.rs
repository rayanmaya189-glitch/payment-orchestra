//! SettlementBatch entity — `settlement_batches` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `settlement_batches` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "settlement_batches")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub settlement_batch_id: Uuid,
    pub batch_file_name: String,
    pub status: String,
    pub ingested_at: DateTimeUtc,
    pub total_transactions: i32,
    pub total_amount_minor: i64,
    pub currency: String,
    /// JSONB: serialized Vec<SettlementRecord>
    pub records: Json,
    pub file_checksum: String,
    pub matched_at: Option<DateTimeUtc>,
    pub quarantined_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
