//! PostgreSQL-backed SubscriptionRepository using SeaORM + platform-db entities.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use super::SubscriptionRepository;
use crate::entities::{Entity as SubEntity, ActiveModel as SubActiveModel, Model as SubModel, Column as SubColumn};

/// SeaORM-backed subscription repository.
pub struct PostgresSubscriptionRepository {
    pub db: DatabaseConnection,
}

impl PostgresSubscriptionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SubscriptionRepository for PostgresSubscriptionRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Subscription>, SubscriptionError> {
        let result = SubEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, sub: &mut Subscription) -> Result<(), SubscriptionError> {
        let model = domain_to_model(sub)?;
        let exists = SubEntity::find_by_id(sub.subscription_id)
            .one(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?
            .is_some();
        if exists {
            SubEntity::update(SubActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?;
        } else {
            SubEntity::insert(SubActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        let models = SubEntity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(SubColumn::Status.eq("active"))
                    .add(SubColumn::Status.eq("past_due")),
            )
            .all(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let models = SubEntity::find()
            .filter(SubColumn::CustomerId.eq(customer_id))
            .all(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let models = SubEntity::find()
            .filter(SubColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

// ─── Domain ↔ Model conversion ───────────────────────────────────────────────

fn domain_to_model(sub: &Subscription) -> Result<SubModel, SubscriptionError> {
    use serde_json::json;
    Ok(SubModel {
        subscription_id: sub.subscription_id,
        operator_id: sub.operator_id,
        customer_id: sub.customer_id,
        plan_id: sub.plan_id.clone(),
        plan_amount_minor_units: sub.plan_amount_minor_units,
        currency: sub.currency.clone(),
        status: sub.status.to_string(),
        current_period_start: sub.current_period_start,
        current_period_end: sub.current_period_end,
        billing_interval_days: sub.billing_interval_days,
        payment_method_token_id: sub.payment_method_token_id,
        dunning_retry_count: sub.dunning_retry_count,
        max_dunning_retries: sub.max_dunning_retries,
        billing_cycles: json!(sub.billing_cycles),
        dunning_retries: json!(sub.dunning_retries),
        created_at: sub.created_at,
        cancelled_at: sub.cancelled_at,
        paused_at: sub.paused_at,
        resumed_at: sub.resumed_at,
    })
}

fn model_to_domain(m: SubModel) -> Result<Subscription, SubscriptionError> {
    Ok(Subscription {
        subscription_id: m.subscription_id,
        operator_id: m.operator_id,
        customer_id: m.customer_id,
        plan_id: m.plan_id,
        plan_amount_minor_units: m.plan_amount_minor_units,
        currency: m.currency,
        status: m.status.parse().map_err(|e: String| SubscriptionError::DatabaseError(e))?,
        current_period_start: m.current_period_start,
        current_period_end: m.current_period_end,
        billing_interval_days: m.billing_interval_days,
        payment_method_token_id: m.payment_method_token_id,
        dunning_retry_count: m.dunning_retry_count,
        max_dunning_retries: m.max_dunning_retries,
        billing_cycles: serde_json::from_value(m.billing_cycles)
            .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?,
        dunning_retries: serde_json::from_value(m.dunning_retries)
            .map_err(|e| SubscriptionError::DatabaseError(e.to_string()))?,
        created_at: m.created_at,
        cancelled_at: m.cancelled_at,
        paused_at: m.paused_at,
        resumed_at: m.resumed_at,
        pending_events: Vec::new(),
    })
}
