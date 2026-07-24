use async_trait::async_trait;
use chrono::Utc;

use super::{AnalyticsQueryHandler, QueryHandler};
use crate::domain::*;
use crate::repository::*;

#[async_trait]
impl<R: AnalyticsRepository + Send + Sync> QueryHandler for AnalyticsQueryHandler<R> {
    async fn authorization_rates(&self, _query: AuthRateQuery) -> Result<Vec<AuthRateRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn decline_reasons(&self, _query: DeclineReasonQuery) -> Result<Vec<DeclineReasonRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn settlement_status(&self, _date_range: DateRangeFilter) -> Result<Vec<SettlementStatusRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn fee_analysis(&self, _query: FeeAnalysisQuery) -> Result<Vec<FeeAnalysisRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn chargeback_trends(&self, _query: ChargebackTrendQuery) -> Result<Vec<ChargebackTrendRow>, AnalyticsError> {
        unreachable!("implemented in fraud module")
    }

    async fn scheme_compliance(&self) -> Result<Vec<SchemeComplianceRow>, AnalyticsError> {
        unreachable!("implemented in fraud module")
    }

    async fn fraud_analysis(&self, _query: FraudAnalysisQuery) -> Result<Vec<FraudAnalysisRow>, AnalyticsError> {
        unreachable!("implemented in fraud module")
    }

    async fn revenue_recovery(&self, date_range: DateRangeFilter) -> Result<Vec<RevenueRecoveryRow>, AnalyticsError> {
        let routing_events = self
            .repo
            .get_events_by_type("RoutingDecision", date_range.start, date_range.end)
            .await?;

        let mut failover_count: u64 = 0;
        let mut recovered_amount: i64 = 0;

        for event in &routing_events {
            if event.failover_routed.unwrap_or(false) {
                failover_count += 1;
                recovered_amount += event.amount_minor_units.unwrap_or(0);
            }
        }

        let estimated_savings = (recovered_amount as f64 * 0.02) as i64;

        Ok(vec![
            RevenueRecoveryRow {
                period: format!(
                    "{} - {}",
                    date_range.start.format("%Y-%m-%d"),
                    date_range.end.format("%Y-%m-%d")
                ),
                failover_count,
                recovered_amount_minor_units: recovered_amount,
                estimated_savings_minor_units: estimated_savings,
                currency: "USD".into(),
            },
        ])
    }

    async fn health_check(&self, staleness_threshold_seconds: i64) -> Result<(), AnalyticsError> {
        match self.repo.last_ingested_at().await {
            Some(last) => {
                let elapsed = (Utc::now() - last).num_seconds();
                if elapsed > staleness_threshold_seconds {
                    return Err(AnalyticsError::StaleData(elapsed));
                }
                Ok(())
            }
            None => {
                Ok(())
            }
        }
    }
}
