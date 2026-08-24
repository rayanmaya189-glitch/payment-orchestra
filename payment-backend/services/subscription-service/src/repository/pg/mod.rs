//! PostgreSQL-backed Subscription repository using SeaORM CRUD.
//!
//! Converts between the domain Subscription model (with SubscriptionStatus enum,
//! Vec<BillingCycle> and Vec<DunningRetry> JSONB) and the flat SeaORM entity model.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::{
    ActiveModel as SubscriptionActiveModel, Column as SubscriptionColumn,
    Entity as SubscriptionEntity, Model as SubscriptionModel,
};

/// PostgreSQL-backed repository implementing SubscriptionRepository.
#[derive(Clone)]
pub struct PostgresSubscriptionRepository {
    pub db: sea_orm::DatabaseConnection,
}

impl PostgresSubscriptionRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn subscription_domain_to_model(sub: &Subscription) -> Result<SubscriptionModel, SubscriptionError> {
    let billing_cycles_json = serde_json::to_value(&sub.billing_cycles)
        .map_err(|e| SubscriptionError::DatabaseError(format!("Serialize billing_cycles: {e}")))?;
    let dunning_retries_json = serde_json::to_value(&sub.dunning_retries)
        .map_err(|e| SubscriptionError::DatabaseError(format!("Serialize dunning_retries: {e}")))?;

    Ok(SubscriptionModel {
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
        billing_cycles: billing_cycles_json,
        dunning_retries: dunning_retries_json,
        created_at: sub.created_at,
        cancelled_at: sub.cancelled_at,
        paused_at: sub.paused_at,
        resumed_at: sub.resumed_at,
    })
}

fn subscription_model_to_domain(m: SubscriptionModel) -> Result<Subscription, SubscriptionError> {
    let billing_cycles: Vec<BillingCycle> = serde_json::from_value(m.billing_cycles)
        .map_err(|e| SubscriptionError::DatabaseError(format!("Deserialize billing_cycles: {e}")))?;
    let dunning_retries: Vec<DunningRetry> = serde_json::from_value(m.dunning_retries)
        .map_err(|e| SubscriptionError::DatabaseError(format!("Deserialize dunning_retries: {e}")))?;
    let status: SubscriptionStatus = m.status
        .parse()
        .map_err(|e: String| SubscriptionError::DatabaseError(format!("Parse status: {e}")))?;

    Ok(Subscription {
        subscription_id: m.subscription_id,
        operator_id: m.operator_id,
        customer_id: m.customer_id,
        plan_id: m.plan_id,
        plan_amount_minor_units: m.plan_amount_minor_units,
        currency: m.currency,
        status,
        current_period_start: m.current_period_start,
        current_period_end: m.current_period_end,
        billing_interval_days: m.billing_interval_days,
        payment_method_token_id: m.payment_method_token_id,
        dunning_retry_count: m.dunning_retry_count,
        max_dunning_retries: m.max_dunning_retries,
        billing_cycles,
        dunning_retries,
        created_at: m.created_at,
        cancelled_at: m.cancelled_at,
        paused_at: m.paused_at,
        resumed_at: m.resumed_at,
        pending_events: Vec::new(),
    })
}

// ─── SubscriptionRepository Trait Implementation ─────────────────────────────

use crate::repository::SubscriptionRepository;

#[async_trait]
impl SubscriptionRepository for PostgresSubscriptionRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Subscription>, SubscriptionError> {
        let result = SubscriptionEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(subscription_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, subscription: &mut Subscription) -> Result<(), SubscriptionError> {
        let model = subscription_domain_to_model(subscription)?;

        let exists = SubscriptionEntity::find_by_id(subscription.subscription_id)
            .one(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(format!("Database error: {e}")))?
            .is_some();

        if exists {
            SubscriptionEntity::update(SubscriptionActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| SubscriptionError::DatabaseError(format!("Database error: {e}")))?;
        } else {
            SubscriptionEntity::insert(SubscriptionActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| SubscriptionError::DatabaseError(format!("Database error: {e}")))?;
        }

        Ok(())
    }

    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        use chrono::Utc;

        let results = SubscriptionEntity::find()
            .filter(SubscriptionColumn::Status.eq("active"))
            .filter(SubscriptionColumn::CurrentPeriodEnd.lte(Utc::now()))
            .all(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(subscription_model_to_domain)
            .collect()
    }

    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let results = SubscriptionEntity::find()
            .filter(SubscriptionColumn::CustomerId.eq(customer_id))
            .all(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(subscription_model_to_domain)
            .collect()
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let results = SubscriptionEntity::find()
            .filter(SubscriptionColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| SubscriptionError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(subscription_model_to_domain)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn sample_subscription() -> Subscription {
        let now = Utc::now();
        Subscription {
            subscription_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            plan_id: "plan_basic".into(),
            plan_amount_minor_units: 999,
            currency: "USD".into(),
            status: SubscriptionStatus::Active,
            current_period_start: now,
            current_period_end: now + Duration::days(30),
            billing_interval_days: 30,
            payment_method_token_id: None,
            dunning_retry_count: 0,
            max_dunning_retries: 3,
            billing_cycles: vec![BillingCycle {
                billing_cycle_id: Uuid::now_v7(),
                period_start: now,
                period_end: now + Duration::days(30),
                status: BillingCycleStatus::Succeeded,
                payment_intent_id: Some(Uuid::now_v7()),
                idempotency_key: "test-key-0".into(),
                created_at: now,
            }],
            dunning_retries: vec![],
            created_at: now,
            cancelled_at: None,
            paused_at: None,
            resumed_at: None,
            pending_events: Vec::new(),
        }
    }

    #[test]
    fn test_subscription_domain_entity_roundtrip() {
        let sub = sample_subscription();

        let model = subscription_domain_to_model(&sub).unwrap();
        let roundtrip = subscription_model_to_domain(model).unwrap();

        assert_eq!(roundtrip.subscription_id, sub.subscription_id);
        assert_eq!(roundtrip.plan_id, "plan_basic");
        assert_eq!(roundtrip.status, SubscriptionStatus::Active);
        assert_eq!(roundtrip.plan_amount_minor_units, 999);
        assert_eq!(roundtrip.currency, "USD");
        assert_eq!(roundtrip.billing_interval_days, 30);
        assert_eq!(roundtrip.dunning_retry_count, 0);
        assert_eq!(roundtrip.max_dunning_retries, 3);
        assert_eq!(roundtrip.billing_cycles.len(), 1);
        assert_eq!(roundtrip.billing_cycles[0].status, BillingCycleStatus::Succeeded);
        assert!(roundtrip.dunning_retries.is_empty());
        assert!(roundtrip.cancelled_at.is_none());
        assert!(roundtrip.pending_events.is_empty());
    }

    #[test]
    fn test_subscription_status_serialization() {
        assert_eq!(SubscriptionStatus::Active.to_string(), "active");
        assert_eq!(SubscriptionStatus::PastDue.to_string(), "past_due");
        assert_eq!(SubscriptionStatus::Cancelled.to_string(), "cancelled");
        assert_eq!(SubscriptionStatus::Paused.to_string(), "paused");

        assert_eq!("active".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::Active);
        assert_eq!("past_due".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::PastDue);
        assert_eq!("cancelled".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::Cancelled);
        assert_eq!("paused".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::Paused);
    }
}
