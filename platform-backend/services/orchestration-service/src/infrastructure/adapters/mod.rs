pub mod event_store;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait};
use uuid::Uuid;

use crate::domain::aggregates::{PaymentIntent, RoutingAttempt, RoutingPolicy};
use crate::infrastructure::entities::{payment_intent, routing_attempt, routing_policy};
use crate::infrastructure::repository::{PaymentIntentRepository, RoutingPolicyRepository};
use platform_error::PlatformError;

pub struct PostgresPaymentIntentRepository {
    db: DatabaseConnection,
}

impl PostgresPaymentIntentRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentIntentRepository for PostgresPaymentIntentRepository {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentIntent>, PlatformError> {
        let model = payment_intent::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, intent: &PaymentIntent) -> Result<(), PlatformError> {
        let active_model: payment_intent::ActiveModel = intent.clone().into();

        let existing = payment_intent::Entity::find_by_id(intent.payment_intent_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }

    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<PaymentIntent>, PlatformError> {
        let model = payment_intent::Entity::find()
            .filter(payment_intent::Column::IdempotencyKey.eq(key))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save_attempt(&self, attempt: &RoutingAttempt) -> Result<(), PlatformError> {
        let active_model: routing_attempt::ActiveModel = attempt.clone().into();

        let existing = routing_attempt::Entity::find_by_id(attempt.attempt_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }

    async fn load_attempts(&self, payment_intent_id: Uuid) -> Result<Vec<RoutingAttempt>, PlatformError> {
        let models = routing_attempt::Entity::find()
            .filter(routing_attempt::Column::PaymentIntentId.eq(payment_intent_id))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }
}

pub struct PostgresRoutingPolicyRepository {
    db: DatabaseConnection,
}

impl PostgresRoutingPolicyRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RoutingPolicyRepository for PostgresRoutingPolicyRepository {
    async fn load_active_for_operator(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, PlatformError> {
        let model = routing_policy::Entity::find()
            .filter(routing_policy::Column::OperatorId.eq(operator_id))
            .filter(routing_policy::Column::Status.eq("active"))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, policy: &RoutingPolicy) -> Result<(), PlatformError> {
        let active_model: routing_policy::ActiveModel = policy.clone().into();

        let existing = routing_policy::Entity::find_by_id(policy.routing_policy_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }
}
