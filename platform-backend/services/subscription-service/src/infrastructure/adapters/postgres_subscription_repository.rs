use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::domain::aggregates::Subscription;
use crate::domain::rules::{
    PaginationParams, SubscriptionFilter, SubscriptionRepository,
};
use crate::domain::value_objects::{SubscriptionInterval, SubscriptionStatus};
use crate::infrastructure::entities::subscription_entity;
use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

pub struct PostgresSubscriptionRepository {
    db: DatabaseConnection,
}

impl PostgresSubscriptionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SubscriptionRepository for PostgresSubscriptionRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Subscription>, PlatformError> {
        let model = subscription_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;
        Ok(model.map(|m| m.into()))
    }

    async fn save(&self, sub: &Subscription) -> Result<(), PlatformError> {
        let existing = subscription_entity::Entity::find_by_id(sub.subscription_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        if let Some(model) = existing {
            let mut active = subscription_entity::ActiveModel::from(model);
            active.status = Set(sub.status.as_str().to_string());
            active.amount_minor_units = Set(sub.amount.amount_minor_units);
            active.current_period_start = Set(sub.current_period_start.into());
            active.current_period_end = Set(sub.current_period_end.into());
            active.retry_count = Set(sub.retry_count);
            active.failed_payment_intent_id = Set(sub.failed_payment_intent_id);
            active.canceled_at = Set(sub.canceled_at.map(|dt| dt.into()));
            active.updated_at = Set(sub.updated_at.into());
            active.update(&self.db).await.map_err(|e| {
                PlatformError::Internal(format!("DB update failed: {e}"))
            })?;
        } else {
            let active = subscription_entity::ActiveModel {
                subscription_id: Set(sub.subscription_id),
                operator_id: Set(sub.operator_id),
                customer_id: Set(sub.customer_id),
                status: Set(sub.status.as_str().to_string()),
                amount_minor_units: Set(sub.amount.amount_minor_units),
                currency: Set(sub.amount.currency.0.clone()),
                interval: Set(sub.interval.as_str().to_string()),
                interval_count: Set(sub.interval_count),
                current_period_start: Set(sub.current_period_start.into()),
                current_period_end: Set(sub.current_period_end.into()),
                trial_period_days: Set(sub.trial_period_days),
                payment_method_token_id: Set(sub.payment_method_token_id.clone()),
                failed_payment_intent_id: Set(sub.failed_payment_intent_id),
                retry_count: Set(sub.retry_count),
                max_retries: Set(sub.max_retries),
                canceled_at: Set(sub.canceled_at.map(|dt| dt.into())),
                created_at: Set(sub.created_at.into()),
                updated_at: Set(sub.updated_at.into()),
            };
            active.insert(&self.db).await.map_err(|e| {
                PlatformError::Internal(format!("DB insert failed: {e}"))
            })?;
        }
        Ok(())
    }

    async fn list_by_customer(
        &self,
        customer_id: Uuid,
    ) -> Result<Vec<Subscription>, PlatformError> {
        let models = subscription_entity::Entity::find()
            .filter(subscription_entity::Column::CustomerId.eq(customer_id))
            .order_by_desc(subscription_entity::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;
        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn list_by_operator(
        &self,
        operator_id: Uuid,
        filter: &SubscriptionFilter,
        pagination: &PaginationParams,
    ) -> Result<Vec<Subscription>, PlatformError> {
        let mut query = subscription_entity::Entity::find()
            .filter(subscription_entity::Column::OperatorId.eq(operator_id));

        if let Some(ref status) = filter.status {
            query = query.filter(subscription_entity::Column::Status.eq(status.as_str()));
        }

        if let Some(customer_id) = filter.customer_id {
            query = query.filter(subscription_entity::Column::CustomerId.eq(customer_id));
        }

        if let Some(min_amount) = filter.min_amount {
            query = query.filter(subscription_entity::Column::AmountMinorUnits.gte(min_amount));
        }

        if let Some(max_amount) = filter.max_amount {
            query = query.filter(subscription_entity::Column::AmountMinorUnits.lte(max_amount));
        }

        let models = query
            .order_by_desc(subscription_entity::Column::CreatedAt)
            .offset(pagination.offset as u64)
            .limit(pagination.limit as u64)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn find_due_subscriptions(&self) -> Result<Vec<Subscription>, PlatformError> {
        let now = chrono::Utc::now();
        let models = subscription_entity::Entity::find()
            .filter(
                subscription_entity::Column::Status
                    .is_in(vec!["active", "past_due", "trialing"]),
            )
            .filter(subscription_entity::Column::CurrentPeriodEnd.lte(now))
            .order_by_asc(subscription_entity::Column::CurrentPeriodEnd)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;
        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn find_canceled_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Subscription>, PlatformError> {
        let models = subscription_entity::Entity::find()
            .filter(subscription_entity::Column::Status.eq("canceled"))
            .filter(subscription_entity::Column::CanceledAt.gte(since))
            .order_by_desc(subscription_entity::Column::CanceledAt)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;
        Ok(models.into_iter().map(|m| m.into()).collect())
    }
}

impl From<subscription_entity::Model> for Subscription {
    fn from(m: subscription_entity::Model) -> Self {
        Subscription {
            subscription_id: m.subscription_id,
            operator_id: m.operator_id,
            customer_id: m.customer_id,
            status: SubscriptionStatus::from_str(&m.status),
            amount: Money {
                amount_minor_units: m.amount_minor_units,
                currency: CurrencyCode::new(&m.currency).unwrap(),
            },
            interval: SubscriptionInterval::from_str(&m.interval),
            interval_count: m.interval_count,
            current_period_start: m.current_period_start.into(),
            current_period_end: m.current_period_end.into(),
            trial_period_days: m.trial_period_days,
            payment_method_token_id: m.payment_method_token_id,
            failed_payment_intent_id: m.failed_payment_intent_id,
            retry_count: m.retry_count,
            max_retries: m.max_retries,
            canceled_at: m.canceled_at.map(|dt| dt.into()),
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
            uncommitted_events: Vec::new(),
        }
    }
}
