use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use crate::application::commands::*;
use crate::domain::aggregates::PaymentAnalytics;
use crate::domain::rules::AnalyticsRepository;
use platform_error::PlatformError;

pub struct AnalyticsServiceImpl { repo: Box<dyn AnalyticsRepository>, db: DatabaseConnection }
impl AnalyticsServiceImpl { pub fn new(repo: Box<dyn AnalyticsRepository>, db: DatabaseConnection) -> Self { Self { repo, db } } }

#[async_trait]
pub trait AnalyticsService: Send + Sync {
    async fn get_payment_analytics(&self, cmd: GetPaymentAnalyticsCommand) -> Result<PaymentAnalytics, PlatformError>;
}

#[async_trait]
impl AnalyticsService for AnalyticsServiceImpl {
    async fn get_payment_analytics(&self, cmd: GetPaymentAnalyticsCommand) -> Result<PaymentAnalytics, PlatformError> {
        self.repo.get_payment_analytics(&cmd.operator_id.to_string(), &cmd.start, &cmd.end).await
    }
}
