//! Principal entity — `principals` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `principals` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "principals")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub id: Uuid,
    pub principal_type: String,
    pub email: Option<String>,
    pub password_hash: Option<Vec<u8>>,
    pub mfa_enrolled: bool,
    pub mfa_method: Option<String>,
    pub status: String,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub last_login_at: Option<DateTimeUtc>,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
