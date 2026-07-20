use sea_orm::entity::prelude::*;
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "settlement_batch")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)] pub batch_id: Uuid,
    pub operator_id: Uuid, pub connector_id: String, pub status: String,
    pub total_amount_minor_units: i64, pub currency: String,
    pub total_records: i32, pub matched_count: i32, pub unmatched_count: i32, pub exception_count: i32,
    pub period_start: String, pub period_end: String,
    pub exceptions: Option<Json>,
    pub polled_at: Option<DateTimeWithTimeZone>, pub matched_at: Option<DateTimeWithTimeZone>,
    pub settled_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone, pub updated_at: DateTimeWithTimeZone,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
