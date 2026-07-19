use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::RoutingAttempt;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "routing_attempt")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub attempt_id: Uuid,
    pub payment_intent_id: Uuid,
    pub attempt_number: i32,
    pub acquirer_link_id: Uuid,
    pub connector_id: String,
    pub status: String,
    pub decline_reason: Option<String>,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
    pub attempted_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::payment_intent_entity::Entity",
        from = "Column::PaymentIntentId",
        to = "super::payment_intent_entity::Column::PaymentIntentId"
    )]
    PaymentIntent,
}

impl Related<super::payment_intent_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaymentIntent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> RoutingAttempt {
        RoutingAttempt {
            attempt_id: self.attempt_id,
            payment_intent_id: self.payment_intent_id,
            attempt_number: self.attempt_number,
            acquirer_link_id: self.acquirer_link_id,
            connector_id: self.connector_id.clone(),
            status: self.status.clone(),
            decline_reason: self.decline_reason.clone(),
            acquirer_reference: self.acquirer_reference.clone(),
            latency_ms: self.latency_ms,
            attempted_at: self.attempted_at.into(),
        }
    }
}

impl From<RoutingAttempt> for ActiveModel {
    fn from(a: RoutingAttempt) -> Self {
        Self {
            attempt_id: sea_orm::Set(a.attempt_id),
            payment_intent_id: sea_orm::Set(a.payment_intent_id),
            attempt_number: sea_orm::Set(a.attempt_number),
            acquirer_link_id: sea_orm::Set(a.acquirer_link_id),
            connector_id: sea_orm::Set(a.connector_id),
            status: sea_orm::Set(a.status),
            decline_reason: sea_orm::Set(a.decline_reason),
            acquirer_reference: sea_orm::Set(a.acquirer_reference),
            latency_ms: sea_orm::Set(a.latency_ms),
            attempted_at: sea_orm::Set(a.attempted_at.into()),
        }
    }
}
