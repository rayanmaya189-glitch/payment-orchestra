use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "route_config")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub route_id: Uuid,
    pub path_prefix: String,
    pub target_service: String,
    pub target_url: String,
    pub auth_required: bool,
    pub rate_limit: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
