//! PostgreSQL-backed PaymentIntentRepository using SeaORM CRUD with JSONB columns.
//!
//! PaymentIntent aggregates are stored in the `payment_intents` table with
//! routing_attempts serialized as JSONB. This is an alternative to the
//! event-sourced repository in `event_sourced.rs`.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::payment_intent::{
    ActiveModel as PaymentIntentActiveModel, Column as PaymentIntentColumn,
    Entity as PaymentIntentEntity, Model as PaymentIntentModel,
};
use super::PostgresOrchestrationRepository;
use crate::repository::PaymentIntentRepository;

#[async_trait]
impl PaymentIntentRepository for PostgresOrchestrationRepository {
    async fn load_payment_intent(&self, id: Uuid) -> Result<Option<PaymentIntent>, OrchestrationError> {
        let result = PaymentIntentEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save_payment_intent(&self, intent: &mut PaymentIntent) -> Result<(), OrchestrationError> {
        let routing_attempts_json = serde_json::to_value(&intent.routing_attempts)
            .map_err(|e| OrchestrationError::DatabaseError(format!("Serialize routing_attempts: {}", e)))?;

        let model = PaymentIntentModel {
            payment_intent_id: intent.payment_intent_id,
            operator_id: intent.operator_id,
            amount_minor_units: intent.requested_amount.amount_minor_units,
            currency: intent.currency.clone(),
            status: intent.status.to_string(),
            payment_method_type: "card".to_string(),
            captured_amount_minor: intent.captured_amount.amount_minor_units,
            refunded_amount_minor: intent.refunded_amount.amount_minor_units,
            routing_attempts: routing_attempts_json,
            metadata_json: intent.metadata.as_ref().map(|m| m.to_string()),
            error_message: None,
            created_at: intent.created_at,
            updated_at: intent.updated_at,
        };

        let exists = PaymentIntentEntity::find_by_id(intent.payment_intent_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            PaymentIntentEntity::update(PaymentIntentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        } else {
            PaymentIntentEntity::insert(PaymentIntentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        }

        // Clear pending events — they've been persisted as state changes
        intent.pending_events.clear();
        Ok(())
    }

    async fn list_payment_intents_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        let models = PaymentIntentEntity::find()
            .filter(PaymentIntentColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        let mut intents: Vec<PaymentIntent> = Vec::new();
        for model in models {
            intents.push(model_to_domain(model)?);
        }
        intents.sort_by_key(|a| a.created_at);
        Ok(intents)
    }
}

/// Convert a SeaORM PaymentIntent model to the domain PaymentIntent aggregate.
fn model_to_domain(m: PaymentIntentModel) -> Result<PaymentIntent, OrchestrationError> {
    let routing_attempts: Vec<RoutingAttempt> = serde_json::from_value(m.routing_attempts)
        .unwrap_or_default();

    let metadata: Option<serde_json::Value> = m.metadata_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(PaymentIntent {
        payment_intent_id: m.payment_intent_id,
        operator_id: m.operator_id,
        status: m.status.parse().unwrap_or(PaymentStatus::Created),
        requested_amount: Money {
            amount_minor_units: m.amount_minor_units,
            currency: m.currency.clone(),
        },
        authorized_amount: Money::zero(&m.currency),
        captured_amount: Money {
            amount_minor_units: m.captured_amount_minor,
            currency: m.currency.clone(),
        },
        refunded_amount: Money {
            amount_minor_units: m.refunded_amount_minor,
            currency: m.currency.clone(),
        },
        currency: m.currency.clone(),
        idempotency_key: String::new(),
        payment_method_token_id: None,
        routing_policy_id: None,
        deployment_epoch: 0,
        purpose: PaymentPurpose::Payment,
        metadata,
        source_type: None,
        source_id: None,
        risk_score: None,
        risk_level: None,
        expected_settlement_date: None,
        settlement_cycle: None,
        gateway_profile_id: None,
        gateway_profile_version: None,
        gateway_rotation_strategy: None,
        gateway_selection_reason: None,
        routing_attempts,
        version: 0,
        pending_events: Vec::new(),
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

impl std::str::FromStr for PaymentStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Created" => Ok(PaymentStatus::Created),
            "Authorizing" => Ok(PaymentStatus::Authorizing),
            "Authorized" => Ok(PaymentStatus::Authorized),
            "Capturing" => Ok(PaymentStatus::Capturing),
            "Captured" => Ok(PaymentStatus::Captured),
            "PartiallyCaptured" => Ok(PaymentStatus::PartiallyCaptured),
            "Voided" => Ok(PaymentStatus::Voided),
            "AuthorizationExpired" => Ok(PaymentStatus::AuthorizationExpired),
            "Failed" => Ok(PaymentStatus::Failed),
            "FailedAllRoutes" => Ok(PaymentStatus::FailedAllRoutes),
            "Refunding" => Ok(PaymentStatus::Refunding),
            "Refunded" => Ok(PaymentStatus::Refunded),
            "PartiallyRefunded" => Ok(PaymentStatus::PartiallyRefunded),
            _ => Err(format!("Unknown PaymentStatus: {}", s)),
        }
    }
}
