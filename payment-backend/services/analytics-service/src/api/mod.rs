//! Analytics Service public API layer

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use chrono::{DateTime, Utc};

pub struct AnalyticsApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl AnalyticsApi {
    pub fn new(ch: Box<dyn CommandHandler>, qh: Box<dyn QueryHandler>) -> Self {
        Self {
            command_handler: ch,
            query_handler: qh,
        }
    }

    // -----------------------------------------------------------------------
    // Ingestion
    // -----------------------------------------------------------------------

    pub async fn ingest_event(&self, cmd: IngestAnalyticsEvent) -> Result<AnalyticsEvent, AnalyticsError> {
        self.command_handler.ingest_event(cmd).await
    }

    // -----------------------------------------------------------------------
    // Authorization Rates
    // -----------------------------------------------------------------------

    pub async fn authorization_rates(&self, query: AuthRateQuery) -> Result<Vec<AuthRateRow>, AnalyticsError> {
        self.query_handler.authorization_rates(query).await
    }

    // -----------------------------------------------------------------------
    // Decline Reasons
    // -----------------------------------------------------------------------

    pub async fn decline_reasons(&self, query: DeclineReasonQuery) -> Result<Vec<DeclineReasonRow>, AnalyticsError> {
        self.query_handler.decline_reasons(query).await
    }

    // -----------------------------------------------------------------------
    // Settlement Status
    // -----------------------------------------------------------------------

    pub async fn settlement_status(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<SettlementStatusRow>, AnalyticsError> {
        self.query_handler
            .settlement_status(DateRangeFilter { start, end })
            .await
    }

    // -----------------------------------------------------------------------
    // Fee Analysis
    // -----------------------------------------------------------------------

    pub async fn fee_analysis(&self, query: FeeAnalysisQuery) -> Result<Vec<FeeAnalysisRow>, AnalyticsError> {
        self.query_handler.fee_analysis(query).await
    }

    // -----------------------------------------------------------------------
    // Chargeback Trends
    // -----------------------------------------------------------------------

    pub async fn chargeback_trends(
        &self,
        query: ChargebackTrendQuery,
    ) -> Result<Vec<ChargebackTrendRow>, AnalyticsError> {
        self.query_handler.chargeback_trends(query).await
    }

    // -----------------------------------------------------------------------
    // Scheme Compliance
    // -----------------------------------------------------------------------

    pub async fn scheme_compliance(&self) -> Result<Vec<SchemeComplianceRow>, AnalyticsError> {
        self.query_handler.scheme_compliance().await
    }

    // -----------------------------------------------------------------------
    // Fraud Analysis
    // -----------------------------------------------------------------------

    pub async fn fraud_analysis(&self, query: FraudAnalysisQuery) -> Result<Vec<FraudAnalysisRow>, AnalyticsError> {
        self.query_handler.fraud_analysis(query).await
    }

    // -----------------------------------------------------------------------
    // Revenue Recovery
    // -----------------------------------------------------------------------

    pub async fn revenue_recovery(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<RevenueRecoveryRow>, AnalyticsError> {
        self.query_handler
            .revenue_recovery(DateRangeFilter { start, end })
            .await
    }

    // -----------------------------------------------------------------------
    // Health
    // -----------------------------------------------------------------------

    pub async fn health_check(&self, staleness_threshold_seconds: i64) -> Result<(), AnalyticsError> {
        self.query_handler.health_check(staleness_threshold_seconds).await
    }
}
