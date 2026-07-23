//! PostgreSQL-backed AnalyticsRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::AnalyticsRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as AnalyticsEventActiveModel,
    Column as AnalyticsEventColumn,
    Entity as AnalyticsEventEntity,
    Model as AnalyticsEventModel,
};

pub struct PostgresAnalyticsRepository {
    pub db: DatabaseConnection,
}

impl PostgresAnalyticsRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AnalyticsRepository for PostgresAnalyticsRepository {
    async fn store_event(&self, event: &AnalyticsEvent) -> Result<(), AnalyticsError> {
        let model = domain_to_model(event);
        AnalyticsEventEntity::insert(AnalyticsEventActiveModel::from(model))
            .exec(&self.db)
            .await
            .map_err(|e| AnalyticsError::Unavailable(e.to_string()))?;
        Ok(())
    }

    async fn query_events(
        &self,
        operator_id: Option<Uuid>,
        event_type: Option<&str>,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let mut query = AnalyticsEventEntity::find();

        if let Some(oid) = operator_id {
            query = query.filter(AnalyticsEventColumn::OperatorId.eq(Some(oid)));
        }
        if let Some(et) = event_type {
            query = query.filter(AnalyticsEventColumn::EventType.eq(et));
        }
        if let Some(s) = since {
            query = query.filter(AnalyticsEventColumn::OccurredAt.gte(s));
        }
        if let Some(u) = until {
            query = query.filter(AnalyticsEventColumn::OccurredAt.lte(u));
        }

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| AnalyticsError::Unavailable(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn query_by_acquirer(
        &self,
        acquirer_id: &str,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let mut query = AnalyticsEventEntity::find()
            .filter(AnalyticsEventColumn::AcquirerId.eq(Some(acquirer_id.to_string())));

        if let Some(s) = since {
            query = query.filter(AnalyticsEventColumn::OccurredAt.gte(s));
        }
        if let Some(u) = until {
            query = query.filter(AnalyticsEventColumn::OccurredAt.lte(u));
        }

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| AnalyticsError::Unavailable(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn count_events(&self) -> Result<u64, AnalyticsError> {
        let count = AnalyticsEventEntity::find()
            .count(&self.db)
            .await
            .map_err(|e| AnalyticsError::Unavailable(e.to_string()))?;
        Ok(count)
    }

    async fn latest_ingested_at(&self) -> Result<Option<DateTime<Utc>>, AnalyticsError> {
        let model = AnalyticsEventEntity::find()
            .order_by_desc(AnalyticsEventColumn::IngestedAt)
            .one(&self.db)
            .await
            .map_err(|e| AnalyticsError::Unavailable(e.to_string()))?;
        Ok(model.map(|m| m.ingested_at))
    }
}

fn domain_to_model(e: &AnalyticsEvent) -> AnalyticsEventModel {
    AnalyticsEventModel {
        event_id: e.event_id,
        event_type: e.event_type.clone(),
        payment_intent_id: e.payment_intent_id,
        operator_id: e.operator_id,
        acquirer_id: e.acquirer_id.clone(),
        card_scheme: e.card_scheme.clone(),
        currency: e.currency.clone(),
        amount_minor_units: e.amount_minor_units,
        decline_reason: e.decline_reason.clone(),
        latency_ms: e.latency_ms.map(|v| v as i32),
        acquirer_fee: e.acquirer_fee,
        chargeback_amount: e.chargeback_amount,
        chargeback_reason: e.chargeback_reason.clone(),
        fraud_score: e.fraud_score,
        bin: e.bin.clone(),
        country_code: e.country_code.clone(),
        merchant_id: e.merchant_id.clone(),
        failover_routed: e.failover_routed,
        occurred_at: e.occurred_at,
        ingested_at: e.ingested_at,
    }
}

fn model_to_domain(m: AnalyticsEventModel) -> Result<AnalyticsEvent, AnalyticsError> {
    Ok(AnalyticsEvent {
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
    })
}
