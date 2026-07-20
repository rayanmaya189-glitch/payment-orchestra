use async_trait::async_trait;
use crate::domain::aggregates::PaymentAnalytics;
use platform_error::PlatformError;

#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    async fn get_payment_analytics(&self, operator_id: &str, start: &str, end: &str) -> Result<PaymentAnalytics, PlatformError>;
}
