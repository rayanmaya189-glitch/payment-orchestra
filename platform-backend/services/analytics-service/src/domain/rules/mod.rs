use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::PaymentAnalytics;
use crate::domain::entities::{
    AuthorizationEvent, ConnectorPerformance, DeclineBreakdownEntry, HourlyTrend, OperatorAnalytics,
    VolumeBucket,
};
use crate::domain::value_objects::TimeRange;
use platform_error::PlatformError;

#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    async fn get_payment_analytics(
        &self,
        operator_id: &str,
        start: &str,
        end: &str,
    ) -> Result<PaymentAnalytics, PlatformError>;

    async fn get_operator_analytics(
        &self,
        operator_id: Uuid,
        time_range: &TimeRange,
    ) -> Result<OperatorAnalytics, PlatformError>;

    async fn get_volume_buckets(
        &self,
        operator_id: Uuid,
        time_range: &TimeRange,
    ) -> Result<Vec<VolumeBucket>, PlatformError>;

    async fn get_decline_breakdown(
        &self,
        operator_id: Uuid,
        time_range: &TimeRange,
    ) -> Result<Vec<DeclineBreakdownEntry>, PlatformError>;

    async fn get_connector_performance(
        &self,
        operator_id: Uuid,
        time_range: &TimeRange,
    ) -> Result<Vec<ConnectorPerformance>, PlatformError>;

    async fn get_hourly_trends(
        &self,
        operator_id: Uuid,
        time_range: &TimeRange,
    ) -> Result<Vec<HourlyTrend>, PlatformError>;

    async fn get_authorization_events(
        &self,
        operator_id: Uuid,
        time_range: &TimeRange,
        limit: u32,
    ) -> Result<Vec<AuthorizationEvent>, PlatformError>;
}
