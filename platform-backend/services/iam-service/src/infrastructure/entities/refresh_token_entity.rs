use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "refresh_token")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub client_fingerprint: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub revoke_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::principal_entity::Entity",
        from = "Column::PrincipalId",
        to = "super::principal_entity::Column::Id"
    )]
    Principal,
}

impl Related<super::principal_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Principal.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
