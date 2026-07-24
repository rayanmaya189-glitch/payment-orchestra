use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::PostgresOrchestrationRepository;
use crate::repository::PaymentIntentRepository;
use crate::domain::*;
use crate::entities::payment_intent::{
    Entity as PaymentIntentEntity,
    ActiveModel as PaymentIntentActiveModel,
    Model as PaymentIntentModel,
    Column as PaymentIntentColumn,
};

#[async_trait]
impl PaymentIntentRepository for PostgresOrchestrationRepository {
    async fn load_payment_intent(
        &self,
        id: Uuid,
    ) -> Result<Option<PaymentIntent>, OrchestrationError> {
        let result = PaymentIntentEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(payment_intent_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_payment_intent(
        &self,
        intent: &PaymentIntent,
    ) -> Result<(), OrchestrationError> {
        let model = payment_intent_domain_to_model(intent)?;
        let exists = PaymentIntentEntity::find_by_id(intent.payment_intent_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            PaymentIntentEntity::update(PaymentIntentActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        } else {
            PaymentIntentEntity::insert(PaymentIntentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn list_payment_intents_for_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        let models = PaymentIntentEntity::find()
            .filter(PaymentIntentColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        models.into_iter().map(payment_intent_model_to_domain).collect()
    }
}

// ─── Domain ↔ Model conversion: PaymentIntent ────────────────────────────────

fn payment_intent_domain_to_model(
    pi: &PaymentIntent,
) -> Result<PaymentIntentModel, OrchestrationError> {
    Ok(PaymentIntentModel {
        payment_intent_id: pi.payment_intent_id,
        operator_id: pi.operator_id,
        amount_minor_units: pi.requested_amount.amount_minor_units,
        currency: pi.currency.clone(),
        status: format!("{:?}", pi.status),
        payment_method_type: pi.source_type.clone().unwrap_or_default(),
        captured_amount_minor: pi.captured_amount.amount_minor_units,
        refunded_amount_minor: pi.refunded_amount.amount_minor_units,
        routing_attempts: serde_json::to_value(&pi.routing_attempts)
            .map_err(|e| OrchestrationError::Validation(format!("Serialize routing_attempts: {}", e)))?,
        metadata_json: pi.metadata.as_ref().map(|m| m.to_string()),
        error_message: None,
        created_at: pi.created_at,
        updated_at: pi.updated_at,
    })
}

fn payment_intent_model_to_domain(
    m: PaymentIntentModel,
) -> Result<PaymentIntent, OrchestrationError> {
    let status = match m.status.as_str() {
        "Created" => PaymentStatus::Created,
        "Authorizing" => PaymentStatus::Authorizing,
        "Authorized" => PaymentStatus::Authorized,
        "Capturing" => PaymentStatus::Capturing,
        "Captured" => PaymentStatus::Captured,
        "PartiallyCaptured" => PaymentStatus::PartiallyCaptured,
        "Voided" => PaymentStatus::Voided,
        "AuthorizationExpired" => PaymentStatus::AuthorizationExpired,
        "Failed" => PaymentStatus::Failed,
        "FailedAllRoutes" => PaymentStatus::FailedAllRoutes,
        "Refunding" => PaymentStatus::Refunding,
        "Refunded" => PaymentStatus::Refunded,
        "PartiallyRefunded" => PaymentStatus::PartiallyRefunded,
        _ => return Err(OrchestrationError::Validation(format!("Unknown status: {}", m.status))),
    };

    let routing_attempts: Vec<RoutingAttempt> = serde_json::from_value(m.routing_attempts)
        .map_err(|e| OrchestrationError::Validation(format!("Deserialize routing_attempts: {}", e)))?;

    let metadata: Option<serde_json::Value> = m.metadata_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(PaymentIntent {
        payment_intent_id: m.payment_intent_id,
        operator_id: m.operator_id,
        status,
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
        currency: m.currency,
        idempotency_key: String::new(),
        payment_method_token_id: None,
        routing_policy_id: None,
        deployment_epoch: 0,
        purpose: PaymentPurpose::Payment,
        metadata,
        source_type: Some(m.payment_method_type),
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
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}
