use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "notification")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub notification_id: Uuid,
    pub operator_id: Uuid,
    pub notification_type: String,
    pub status: String,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub template_id: Option<String>,
    pub template_data: Option<Json>,
    pub provider_message_id: Option<String>,
    pub retry_count: i32,
    pub sent_at: Option<DateTimeWithTimeZone>,
    pub delivered_at: Option<DateTimeWithTimeZone>,
    pub failed_at: Option<DateTimeWithTimeZone>,
    pub error_message: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
