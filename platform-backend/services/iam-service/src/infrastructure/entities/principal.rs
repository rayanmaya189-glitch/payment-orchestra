use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::{Principal, PrincipalStatus, PrincipalType};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "principal")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub principal_type: String,
    pub email: Option<String>,
    pub password_hash: Option<Vec<u8>>,
    pub mfa_enrolled: bool,
    pub mfa_method: Option<String>,
    pub status: String,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub last_login_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::api_key_entity::Entity")]
    ApiKey,
    #[sea_orm(has_many = "super::role_assignment_entity::Entity")]
    RoleAssignment,
}

impl Related<super::api_key_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApiKey.def()
    }
}

impl Related<super::role_assignment_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RoleAssignment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> Principal {
        Principal {
            id: self.id,
            principal_type: PrincipalType::from_str(&self.principal_type)
                .expect("DB contains invalid principal_type — data corruption"),
            email: self.email.clone(),
            password_hash: self.password_hash.clone(),
            mfa_enrolled: self.mfa_enrolled,
            mfa_method: self.mfa_method.clone(),
            status: PrincipalStatus::from_str(&self.status)
                .expect("DB contains invalid principal status — data corruption"),
            failed_login_attempts: self.failed_login_attempts,
            locked_until: self.locked_until.map(|dt| dt.into()),
            created_at: self.created_at.into(),
            last_login_at: self.last_login_at.map(|dt| dt.into()),
        }
    }
}

impl From<Principal> for ActiveModel {
    fn from(p: Principal) -> Self {
        Self {
            id: sea_orm::Set(p.id),
            principal_type: sea_orm::Set(p.principal_type.as_str().to_string()),
            email: sea_orm::Set(p.email),
            password_hash: sea_orm::Set(p.password_hash),
            mfa_enrolled: sea_orm::Set(p.mfa_enrolled),
            mfa_method: sea_orm::Set(p.mfa_method),
            status: sea_orm::Set(p.status.as_str().to_string()),
            failed_login_attempts: sea_orm::Set(p.failed_login_attempts),
            locked_until: sea_orm::Set(p.locked_until.map(|dt| dt.into())),
            created_at: sea_orm::Set(p.created_at.into()),
            last_login_at: sea_orm::Set(p.last_login_at.map(|dt| dt.into())),
        }
    }
}
