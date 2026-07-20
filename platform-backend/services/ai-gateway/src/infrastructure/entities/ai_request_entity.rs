use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ai_request")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub request_id: Uuid,
    pub principal_id: Uuid,
    pub prompt: String,
    pub model: String,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub redacted_prompt: Option<String>,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub tokens_used: Option<i32>,
    pub cost_usd: Option<f64>,
    pub created_at: DateTimeWithTimeZone,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
