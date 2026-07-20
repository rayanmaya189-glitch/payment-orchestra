use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_link")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub status: String,
    pub description: String,
    pub merchant_name: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub public_token: String,
    pub metadata: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
