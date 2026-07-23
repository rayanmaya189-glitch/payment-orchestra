//! OnboardingRequest entity — `onboarding_requests` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "onboarding_requests")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub onboarding_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub connector_id: String,
    pub credentials_json: String,
    pub environment: String,
    pub status: String,
    pub test_result: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
