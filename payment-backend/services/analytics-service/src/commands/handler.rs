//! Analytics Service commands

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn ingest_event(&self, cmd: IngestAnalyticsEvent) -> Result<AnalyticsEvent, AnalyticsError>;
}

pub struct AnalyticsCommandHandler<R: AnalyticsRepository> {
    repo: R,
}

impl<R: AnalyticsRepository> AnalyticsCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: AnalyticsRepository + Send + Sync> CommandHandler for AnalyticsCommandHandler<R> {
    async fn ingest_event(&self, cmd: IngestAnalyticsEvent) -> Result<AnalyticsEvent, AnalyticsError> {
        let event = AnalyticsEvent {
            event_id: Uuid::now_v7(),
            event_type: cmd.event_type,
            payment_intent_id: cmd.payment_intent_id,
            operator_id: cmd.operator_id,
            acquirer_id: cmd.acquirer_id,
            card_scheme: cmd.card_scheme,
            currency: cmd.currency,
            amount_minor_units: cmd.amount_minor_units,
            decline_reason: cmd.decline_reason,
            latency_ms: cmd.latency_ms,
            acquirer_fee: cmd.acquirer_fee,
            chargeback_amount: cmd.chargeback_amount,
            chargeback_reason: cmd.chargeback_reason,
            fraud_score: cmd.fraud_score,
            bin: cmd.bin,
            country_code: cmd.country_code,
            merchant_id: cmd.merchant_id,
            failover_routed: cmd.failover_routed,
            occurred_at: chrono::Utc::now(),
            ingested_at: chrono::Utc::now(),
        };

        self.repo.store_event(&event).await?;
        Ok(event)
    }
}
