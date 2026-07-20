use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::SettlementBatch;
use crate::domain::value_objects::SettlementBatchStatus;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "settlement_batch")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub settlement_batch_id: Uuid,
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub file_checksum: String,
    pub file_format: String,
    pub status: String,
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub total_amount_minor_units: i64,
    pub ingested_at: DateTimeWithTimeZone,
    pub processed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> SettlementBatch {
        SettlementBatch {
            settlement_batch_id: self.settlement_batch_id,
            operator_id: self.operator_id,
            acquirer_link_id: self.acquirer_link_id,
            file_checksum: self.file_checksum.clone(),
            file_format: self.file_format.clone(),
            status: SettlementBatchStatus::from_str(&self.status)
                .expect("DB contains invalid settlement batch status"),
            total_records: self.total_records,
            matched_count: self.matched_count,
            unmatched_count: self.unmatched_count,
            total_amount_minor_units: self.total_amount_minor_units,
            ingested_at: self.ingested_at.into(),
            processed_at: self.processed_at.map(|dt| dt.into()),
        }
    }
}

impl From<SettlementBatch> for ActiveModel {
    fn from(b: SettlementBatch) -> Self {
        Self {
            settlement_batch_id: sea_orm::Set(b.settlement_batch_id),
            operator_id: sea_orm::Set(b.operator_id),
            acquirer_link_id: sea_orm::Set(b.acquirer_link_id),
            file_checksum: sea_orm::Set(b.file_checksum),
            file_format: sea_orm::Set(b.file_format),
            status: sea_orm::Set(b.status.as_str().to_string()),
            total_records: sea_orm::Set(b.total_records),
            matched_count: sea_orm::Set(b.matched_count),
            unmatched_count: sea_orm::Set(b.unmatched_count),
            total_amount_minor_units: sea_orm::Set(b.total_amount_minor_units),
            ingested_at: sea_orm::Set(b.ingested_at.into()),
            processed_at: sea_orm::Set(b.processed_at.map(|dt| dt.into())),
        }
    }
}
