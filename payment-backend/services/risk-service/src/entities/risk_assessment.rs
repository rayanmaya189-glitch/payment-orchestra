//! RiskAssessment entity — `risk_assessments` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "risk_assessments")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub risk_assessment_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub payment_intent_id: Uuid,
    pub risk_score: f64,
    pub risk_level: String,
    pub risk_factors: Json,
    pub rule_version: String,
    pub assessed_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
