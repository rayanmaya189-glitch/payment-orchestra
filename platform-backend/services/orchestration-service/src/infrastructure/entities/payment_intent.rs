use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::PaymentIntent;
use crate::domain::value_objects::PaymentPurpose;
use shared_types::{CurrencyCode, Money, PaymentStatus};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_intent")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub status: String,
    pub requested_amount_minor_units: i64,
    pub authorized_amount_minor_units: i64,
    pub captured_amount_minor_units: i64,
    pub refunded_amount_minor_units: i64,
    pub currency: String,
    pub idempotency_key: String,
    pub payment_method_token_id: Option<Uuid>,
    pub routing_policy_id: Option<Uuid>,
    pub purpose: String,
    pub metadata: Option<String>,
    pub gateway_profile_id: Option<Uuid>,
    pub gateway_profile_version: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::routing_attempt_entity::Entity")]
    RoutingAttempt,
}

impl Related<super::routing_attempt_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RoutingAttempt.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> PaymentIntent {
        let currency = CurrencyCode::new(&self.currency).unwrap();
        PaymentIntent {
            payment_intent_id: self.payment_intent_id,
            operator_id: self.operator_id,
            status: PaymentStatus::from_str(&self.status),
            requested_amount: Money {
                amount_minor_units: self.requested_amount_minor_units,
                currency: currency.clone(),
            },
            authorized_amount: Money {
                amount_minor_units: self.authorized_amount_minor_units,
                currency: currency.clone(),
            },
            captured_amount: Money {
                amount_minor_units: self.captured_amount_minor_units,
                currency: currency.clone(),
            },
            refunded_amount: Money {
                amount_minor_units: self.refunded_amount_minor_units,
                currency: currency.clone(),
            },
            idempotency_key: self.idempotency_key.clone(),
            payment_method_token_id: self.payment_method_token_id,
            routing_policy_id: self.routing_policy_id,
            purpose: PaymentPurpose::from_str(&self.purpose),
            metadata: self.metadata.as_ref().and_then(|m| serde_json::from_str(m).ok()),
            gateway_profile_id: self.gateway_profile_id,
            gateway_profile_version: self.gateway_profile_version,
            created_at: self.created_at.into(),
            updated_at: self.updated_at.into(),
        }
    }
}

impl From<PaymentIntent> for ActiveModel {
    fn from(p: PaymentIntent) -> Self {
        Self {
            payment_intent_id: sea_orm::Set(p.payment_intent_id),
            operator_id: sea_orm::Set(p.operator_id),
            status: sea_orm::Set(p.status.as_str().to_string()),
            requested_amount_minor_units: sea_orm::Set(p.requested_amount.amount_minor_units),
            authorized_amount_minor_units: sea_orm::Set(p.authorized_amount.amount_minor_units),
            captured_amount_minor_units: sea_orm::Set(p.captured_amount.amount_minor_units),
            refunded_amount_minor_units: sea_orm::Set(p.refunded_amount.amount_minor_units),
            currency: sea_orm::Set(p.requested_amount.currency.0),
            idempotency_key: sea_orm::Set(p.idempotency_key),
            payment_method_token_id: sea_orm::Set(p.payment_method_token_id),
            routing_policy_id: sea_orm::Set(p.routing_policy_id),
            purpose: sea_orm::Set(p.purpose.as_str().to_string()),
            metadata: sea_orm::Set(p.metadata.map(|m| serde_json::to_string(&m).unwrap())),
            gateway_profile_id: sea_orm::Set(p.gateway_profile_id),
            gateway_profile_version: sea_orm::Set(p.gateway_profile_version),
            created_at: sea_orm::Set(p.created_at.into()),
            updated_at: sea_orm::Set(p.updated_at.into()),
        }
    }
}
