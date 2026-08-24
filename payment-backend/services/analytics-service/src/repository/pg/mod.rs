//! PostgreSQL repository for Analytics Service — BC-15.
//!
//! Persists [`AnalyticsEvent`] to the `analytics_events` table.
//! Pure query-side read model; no event sourcing.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::domain::*;
use crate::entities::analytics_event::{self, Entity as AnalyticsEventEntity, Column as AnalyticsEventColumn};
use crate::repository::AnalyticsRepository;

#[derive(Clone)]
pub struct PostgresAnalyticsStore {
    db: sea_orm::DatabaseConnection,
}

impl PostgresAnalyticsStore {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AnalyticsRepository for PostgresAnalyticsStore {
    async fn store_event(&self, event: &AnalyticsEvent) -> Result<(), AnalyticsError> {
        let model = domain_to_model(event);

        analytics_event::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(analytics_event::Column::EventId)
                    .update_columns([
                        analytics_event::Column::EventType,
                        analytics_event::Column::IngestedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| AnalyticsError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let results = AnalyticsEventEntity::find()
            .filter(AnalyticsEventColumn::OccurredAt.gte(start))
            .filter(AnalyticsEventColumn::OccurredAt.lte(end))
            .order_by_asc(AnalyticsEventColumn::OccurredAt)
            .all(&self.db)
            .await
            .map_err(|e| AnalyticsError::DatabaseError(e.to_string()))?;

        Ok(results.into_iter().map(model_to_domain).collect())
    }

    async fn get_events_by_type(&self, event_type: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let results = AnalyticsEventEntity::find()
            .filter(AnalyticsEventColumn::EventType.eq(event_type))
            .filter(AnalyticsEventColumn::OccurredAt.gte(start))
            .filter(AnalyticsEventColumn::OccurredAt.lte(end))
            .order_by_asc(AnalyticsEventColumn::OccurredAt)
            .all(&self.db)
            .await
            .map_err(|e| AnalyticsError::DatabaseError(e.to_string()))?;

        Ok(results.into_iter().map(model_to_domain).collect())
    }

    async fn get_events_by_acquirer(&self, acquirer_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let results = AnalyticsEventEntity::find()
            .filter(AnalyticsEventColumn::AcquirerId.eq(acquirer_id))
            .filter(AnalyticsEventColumn::OccurredAt.gte(start))
            .filter(AnalyticsEventColumn::OccurredAt.lte(end))
            .order_by_asc(AnalyticsEventColumn::OccurredAt)
            .all(&self.db)
            .await
            .map_err(|e| AnalyticsError::DatabaseError(e.to_string()))?;

        Ok(results.into_iter().map(model_to_domain).collect())
    }

    async fn last_ingested_at(&self) -> Option<DateTime<Utc>> {
        let result = AnalyticsEventEntity::find()
            .order_by_desc(AnalyticsEventColumn::IngestedAt)
            .one(&self.db)
            .await
            .ok()??;

        Some(result.ingested_at)
    }

    async fn event_count(&self) -> u64 {
        AnalyticsEventEntity::find()
            .count(&self.db)
            .await
            .unwrap_or(0)
    }
}

// ─── Domain <-> Entity conversion helpers ─────────────────────────────────────

fn domain_to_model(event: &AnalyticsEvent) -> analytics_event::ActiveModel {
    analytics_event::ActiveModel {
        event_id: sea_orm::ActiveValue::Set(event.event_id),
        event_type: sea_orm::ActiveValue::Set(event.event_type.clone()),
        payment_intent_id: sea_orm::ActiveValue::Set(event.payment_intent_id),
        operator_id: sea_orm::ActiveValue::Set(event.operator_id),
        acquirer_id: sea_orm::ActiveValue::Set(event.acquirer_id.clone()),
        card_scheme: sea_orm::ActiveValue::Set(event.card_scheme.clone()),
        currency: sea_orm::ActiveValue::Set(event.currency.clone()),
        amount_minor_units: sea_orm::ActiveValue::Set(event.amount_minor_units),
        decline_reason: sea_orm::ActiveValue::Set(event.decline_reason.clone()),
        latency_ms: sea_orm::ActiveValue::Set(event.latency_ms.map(|v| v as i32)),
        acquirer_fee: sea_orm::ActiveValue::Set(event.acquirer_fee),
        chargeback_amount: sea_orm::ActiveValue::Set(event.chargeback_amount),
        chargeback_reason: sea_orm::ActiveValue::Set(event.chargeback_reason.clone()),
        fraud_score: sea_orm::ActiveValue::Set(event.fraud_score),
        bin: sea_orm::ActiveValue::Set(event.bin.clone()),
        country_code: sea_orm::ActiveValue::Set(event.country_code.clone()),
        merchant_id: sea_orm::ActiveValue::Set(event.merchant_id.clone()),
        failover_routed: sea_orm::ActiveValue::Set(event.failover_routed),
        occurred_at: sea_orm::ActiveValue::Set(event.occurred_at),
        ingested_at: sea_orm::ActiveValue::Set(event.ingested_at),
    }
}

fn model_to_domain(m: analytics_event::Model) -> AnalyticsEvent {
    AnalyticsEvent {
        event_id: m.event_id,
        event_type: m.event_type,
        payment_intent_id: m.payment_intent_id,
        operator_id: m.operator_id,
        acquirer_id: m.acquirer_id,
        card_scheme: m.card_scheme,
        currency: m.currency,
        amount_minor_units: m.amount_minor_units,
        decline_reason: m.decline_reason,
        latency_ms: m.latency_ms.map(|v| v as u32),
        acquirer_fee: m.acquirer_fee,
        chargeback_amount: m.chargeback_amount,
        chargeback_reason: m.chargeback_reason,
        fraud_score: m.fraud_score,
        bin: m.bin,
        country_code: m.country_code,
        merchant_id: m.merchant_id,
        failover_routed: m.failover_routed,
        occurred_at: m.occurred_at,
        ingested_at: m.ingested_at,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_event() -> AnalyticsEvent {
        AnalyticsEvent {
            event_id: Uuid::now_v7(),
            event_type: "PaymentAuthorized".into(),
            payment_intent_id: Some(Uuid::now_v7()),
            operator_id: Some(Uuid::now_v7()),
            acquirer_id: Some("stripe".into()),
            card_scheme: Some("visa".into()),
            currency: Some("USD".into()),
            amount_minor_units: Some(1000),
            decline_reason: None,
            latency_ms: Some(45),
            acquirer_fee: Some(25),
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: Some("411111".into()),
            country_code: Some("US".into()),
            merchant_id: Some("merchant_1".into()),
            failover_routed: Some(false),
            occurred_at: Utc::now(),
            ingested_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_domain_to_model() {
        let event = sample_event();
        let model = domain_to_model(&event);

        assert_eq!(model.event_id.unwrap(), event.event_id);
        assert_eq!(model.event_type.unwrap(), "PaymentAuthorized");
        assert_eq!(model.currency.unwrap(), Some("USD".into()));
        assert_eq!(model.latency_ms.unwrap(), Some(45i32));
    }

    #[tokio::test]
    async fn test_model_to_domain() {
        let event = sample_event();
        let entity = analytics_event::Model {
            event_id: event.event_id,
            event_type: "PaymentFailed".into(),
            payment_intent_id: event.payment_intent_id,
            operator_id: event.operator_id,
            acquirer_id: event.acquirer_id,
            card_scheme: event.card_scheme,
            currency: event.currency,
            amount_minor_units: event.amount_minor_units,
            decline_reason: Some("insufficient_funds".into()),
            latency_ms: Some(100i32),
            acquirer_fee: None,
            chargeback_amount: None,
            chargeback_reason: None,
            fraud_score: None,
            bin: event.bin,
            country_code: event.country_code,
            merchant_id: event.merchant_id,
            failover_routed: event.failover_routed,
            occurred_at: event.occurred_at,
            ingested_at: event.ingested_at,
        };

        let domain = model_to_domain(entity);
        assert_eq!(domain.event_type, "PaymentFailed");
        assert_eq!(domain.decline_reason, Some("insufficient_funds".into()));
        assert_eq!(domain.latency_ms, Some(100));
        assert!(domain.acquirer_fee.is_none());
    }
}
