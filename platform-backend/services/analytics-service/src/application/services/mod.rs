use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::aggregates::PaymentAnalytics;
use crate::domain::rules::AnalyticsRepository;
use crate::domain::value_objects::TimeRange;
use platform_error::PlatformError;
use platform_middleware::{evaluate_policy, AbacContext};

pub struct AnalyticsServiceImpl {
    repo: Box<dyn AnalyticsRepository>,
    db: DatabaseConnection,
}

impl AnalyticsServiceImpl {
    pub fn new(repo: Box<dyn AnalyticsRepository>, db: DatabaseConnection) -> Self {
        Self { repo, db }
    }

    fn parse_time_range(start: &str, end: &str) -> Result<TimeRange, PlatformError> {
        TimeRange::parse(start, end).map_err(|e| PlatformError::Validation(
            platform_error::ValidationError::MissingField(e),
        ))
    }
}

#[async_trait]
pub trait AnalyticsService: Send + Sync {
    async fn get_payment_analytics(
        &self,
        cmd: GetPaymentAnalyticsCommand,
    ) -> Result<PaymentAnalyticsQueryResult, PlatformError>;

    async fn get_operator_analytics(
        &self,
        cmd: GetOperatorAnalyticsCommand,
    ) -> Result<OperatorAnalyticsQueryResult, PlatformError>;

    async fn get_volume_trend(
        &self,
        cmd: GetVolumeTrendCommand,
    ) -> Result<VolumeTrendQueryResult, PlatformError>;

    async fn get_decline_breakdown(
        &self,
        cmd: GetDeclineBreakdownCommand,
    ) -> Result<DeclineBreakdownQueryResult, PlatformError>;

    async fn get_connector_performance(
        &self,
        cmd: GetConnectorPerformanceCommand,
    ) -> Result<ConnectorPerformanceQueryResult, PlatformError>;

    async fn get_hourly_trends(
        &self,
        cmd: GetVolumeTrendCommand,
    ) -> Result<HourlyTrendQueryResult, PlatformError>;

    async fn get_authorization_events(
        &self,
        cmd: GetAuthorizationEventsCommand,
    ) -> Result<AuthorizationEventsQueryResult, PlatformError>;
}

#[async_trait]
impl AnalyticsService for AnalyticsServiceImpl {
    async fn get_payment_analytics(
        &self,
        cmd: GetPaymentAnalyticsCommand,
    ) -> Result<PaymentAnalyticsQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: cmd.operator_id,
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "analytics".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(cmd.operator_id),
        };
        evaluate_policy(&ctx)?;

        let analytics = self
            .repo
            .get_payment_analytics(
                &cmd.operator_id.to_string(),
                &cmd.start,
                &cmd.end,
            )
            .await?;

        Ok(PaymentAnalyticsQueryResult { analytics })
    }

    async fn get_operator_analytics(
        &self,
        cmd: GetOperatorAnalyticsCommand,
    ) -> Result<OperatorAnalyticsQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: cmd.operator_id,
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "analytics".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(cmd.operator_id),
        };
        evaluate_policy(&ctx)?;

        let time_range = Self::parse_time_range(&cmd.start, &cmd.end)?;
        let analytics = self
            .repo
            .get_operator_analytics(cmd.operator_id, &time_range)
            .await?;

        Ok(OperatorAnalyticsQueryResult { analytics })
    }

    async fn get_volume_trend(
        &self,
        cmd: GetVolumeTrendCommand,
    ) -> Result<VolumeTrendQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: cmd.operator_id,
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "analytics".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(cmd.operator_id),
        };
        evaluate_policy(&ctx)?;

        let time_range = Self::parse_time_range(&cmd.start, &cmd.end)?;
        let buckets = self
            .repo
            .get_volume_buckets(cmd.operator_id, &time_range)
            .await?;

        Ok(VolumeTrendQueryResult { buckets })
    }

    async fn get_decline_breakdown(
        &self,
        cmd: GetDeclineBreakdownCommand,
    ) -> Result<DeclineBreakdownQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: cmd.operator_id,
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "analytics".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(cmd.operator_id),
        };
        evaluate_policy(&ctx)?;

        let time_range = Self::parse_time_range(&cmd.start, &cmd.end)?;
        let breakdown = self
            .repo
            .get_decline_breakdown(cmd.operator_id, &time_range)
            .await?;

        Ok(DeclineBreakdownQueryResult { breakdown })
    }

    async fn get_connector_performance(
        &self,
        cmd: GetConnectorPerformanceCommand,
    ) -> Result<ConnectorPerformanceQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: cmd.operator_id,
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "analytics".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(cmd.operator_id),
        };
        evaluate_policy(&ctx)?;

        let time_range = Self::parse_time_range(&cmd.start, &cmd.end)?;
        let connectors = self
            .repo
            .get_connector_performance(cmd.operator_id, &time_range)
            .await?;

        Ok(ConnectorPerformanceQueryResult { connectors })
    }

    async fn get_hourly_trends(
        &self,
        cmd: GetVolumeTrendCommand,
    ) -> Result<HourlyTrendQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: cmd.operator_id,
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "analytics".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(cmd.operator_id),
        };
        evaluate_policy(&ctx)?;

        let time_range = Self::parse_time_range(&cmd.start, &cmd.end)?;
        let trends = self
            .repo
            .get_hourly_trends(cmd.operator_id, &time_range)
            .await?;

        Ok(HourlyTrendQueryResult { trends })
    }

    async fn get_authorization_events(
        &self,
        cmd: GetAuthorizationEventsCommand,
    ) -> Result<AuthorizationEventsQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: cmd.operator_id,
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "analytics".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(cmd.operator_id),
        };
        evaluate_policy(&ctx)?;

        let time_range = Self::parse_time_range(&cmd.start, &cmd.end)?;
        let events = self
            .repo
            .get_authorization_events(cmd.operator_id, &time_range, cmd.limit)
            .await?;

        Ok(AuthorizationEventsQueryResult { events })
    }
}
