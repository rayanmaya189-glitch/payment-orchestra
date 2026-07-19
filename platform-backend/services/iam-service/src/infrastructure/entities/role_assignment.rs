use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::entities::RoleAssignment;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "role_assignments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub principal_id: Uuid,
    pub role_name: String,
    pub assigned_at: DateTimeWithTimeZone,
    pub assigned_by: Option<Uuid>,
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

impl Model {
    pub fn to_domain(&self) -> RoleAssignment {
        RoleAssignment {
            id: self.id,
            principal_id: self.principal_id,
            role_name: self.role_name.clone(),
            assigned_at: self.assigned_at.into(),
            assigned_by: self.assigned_by,
        }
    }
}
