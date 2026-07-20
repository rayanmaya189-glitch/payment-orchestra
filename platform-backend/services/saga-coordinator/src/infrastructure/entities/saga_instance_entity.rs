use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "saga_instance")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub saga_id: Uuid,
    pub saga_type: String,
    pub status: String,
    pub current_step: i32,
    pub total_steps: i32,
    pub steps: Json,
    pub payload: Json,
    pub compensation_data: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
