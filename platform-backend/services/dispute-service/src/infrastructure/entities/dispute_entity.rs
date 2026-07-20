use sea_orm::entity::prelude::*;
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "dispute")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)] pub dispute_id: Uuid,
    pub payment_intent_id: Uuid, pub operator_id: Uuid, pub status: String,
    pub reason: String, pub reason_code: Option<String>,
    pub disputed_amount_minor_units: i64, pub currency: String,
    pub acquirer_reference: String, pub connector_id: String,
    pub acquirer_dispute_id: Option<String>, pub evidence: Option<Json>,
    pub decision: Option<String>, pub decision_reason: Option<String>,
    pub opened_at: DateTimeWithTimeZone, pub respond_by: Option<DateTimeWithTimeZone>,
    pub resolved_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone, pub updated_at: DateTimeWithTimeZone,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
