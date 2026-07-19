use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::RoutingPolicy;
use crate::domain::value_objects::{FailoverConfig, RoutingRule};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "routing_policy")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub routing_policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub status: String,
    pub rules_json: String,
    pub failover_config_json: String,
    pub max_transaction_amount_minor_units: Option<i64>,
    pub created_at: DateTimeWithTimeZone,
    pub activated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> RoutingPolicy {
        let rules: Vec<RoutingRule> = serde_json::from_str(&self.rules_json).unwrap_or_default();
        let failover_config: FailoverConfig =
            serde_json::from_str(&self.failover_config_json).unwrap_or_default();

        RoutingPolicy {
            routing_policy_id: self.routing_policy_id,
            operator_id: self.operator_id,
            version: self.version,
            status: self.status.clone(),
            rules,
            failover_config,
            created_at: self.created_at.into(),
            activated_at: self.activated_at.map(|dt| dt.into()),
        }
    }
}

impl From<RoutingPolicy> for ActiveModel {
    fn from(p: RoutingPolicy) -> Self {
        Self {
            routing_policy_id: sea_orm::Set(p.routing_policy_id),
            operator_id: sea_orm::Set(p.operator_id),
            version: sea_orm::Set(p.version),
            status: sea_orm::Set(p.status),
            rules_json: sea_orm::Set(serde_json::to_string(&p.rules).unwrap()),
            failover_config_json: sea_orm::Set(serde_json::to_string(&p.failover_config).unwrap()),
            max_transaction_amount_minor_units: sea_orm::Set(None),
            created_at: sea_orm::Set(p.created_at.into()),
            activated_at: sea_orm::Set(p.activated_at.map(|dt| dt.into())),
        }
    }
}
