//! Analytics Service query handlers
//!
//! 8 read endpoints matching the spec:
//! 1. Authorization rates (hourly per acquirer/scheme)
//! 2. Decline reason breakdown
//! 3. Settlement status (matched/unmatched counts)
//! 4. Fee analysis (per acquirer)
//! 5. Chargeback trends (30/90 day by scheme)
//! 6. Scheme compliance (Visa/Mastercard thresholds)
//! 7. Fraud analysis (by BIN, geography, amount)
//! 8. Revenue recovery (via failover)

use async_trait::async_trait;

use crate::domain::*;
use crate::repository::*;

pub mod financial;

// ---------------------------------------------------------------------------
// QueryHandler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn authorization_rates(&self, query: AuthRateQuery) -> Result<Vec<AuthRateRow>, AnalyticsError>;
    async fn decline_reasons(&self, query: DeclineReasonQuery) -> Result<Vec<DeclineReasonRow>, AnalyticsError>;
    async fn settlement_status(&self, date_range: DateRangeFilter) -> Result<Vec<SettlementStatusRow>, AnalyticsError>;
    async fn fee_analysis(&self, query: FeeAnalysisQuery) -> Result<Vec<FeeAnalysisRow>, AnalyticsError>;
    async fn chargeback_trends(&self, query: ChargebackTrendQuery) -> Result<Vec<ChargebackTrendRow>, AnalyticsError>;
    async fn scheme_compliance(&self) -> Result<Vec<SchemeComplianceRow>, AnalyticsError>;
    async fn fraud_analysis(&self, query: FraudAnalysisQuery) -> Result<Vec<FraudAnalysisRow>, AnalyticsError>;
    async fn revenue_recovery(&self, date_range: DateRangeFilter) -> Result<Vec<RevenueRecoveryRow>, AnalyticsError>;
    async fn health_check(&self, staleness_threshold_seconds: i64) -> Result<(), AnalyticsError>;
}

// ---------------------------------------------------------------------------
// AnalyticsQueryHandler
// ---------------------------------------------------------------------------

pub struct AnalyticsQueryHandler<R: AnalyticsRepository> {
    pub(super) repo: R,
}

impl<R: AnalyticsRepository> AnalyticsQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}
