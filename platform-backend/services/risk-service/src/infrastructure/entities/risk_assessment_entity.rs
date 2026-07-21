use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "risk_assessment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub score: f64,
    pub decision: String,
    pub factors: Json,
    pub breakdown: Json,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
