use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ai_query")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub query_id: Uuid,
    pub session_id: Option<String>,
    pub principal_id: Uuid,
    pub query_text: String,
    pub status: String,
    pub answer: Option<String>,
    pub citations: Option<Json>,
    pub confidence: Option<f64>,
    pub tokens_used: Option<i32>,
    pub latency_ms: Option<i64>,
    pub created_at: DateTimeWithTimeZone,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
