use async_trait::async_trait; use uuid::Uuid;
use crate::domain::aggregates::Subscription;
use crate::domain::value_objects::BillingInterval;
use platform_error::PlatformError;

#[async_trait]
pub trait SubscriptionService: Send + Sync {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<SubscriptionResponse, PlatformError>;
    async fn get_subscription(&self, id: Uuid) -> Result<SubscriptionResponse, PlatformError>;
    async fn pause_subscription(&self, id: Uuid) -> Result<(), PlatformError>;
    async fn resume_subscription(&self, id: Uuid) -> Result<(), PlatformError>;
    async fn cancel_subscription(&self, id: Uuid) -> Result<(), PlatformError>;
}

pub struct SubscriptionServiceImpl { db: sea_orm::DatabaseConnection }
impl SubscriptionServiceImpl { pub fn new(db: sea_orm::DatabaseConnection) -> Self { Self { db } } }

pub struct CreateSubscriptionCommand { pub operator_id: Uuid, pub customer_id: Uuid, pub amount: shared_types::Money, pub interval: String }
#[derive(Debug, Clone)]
pub struct SubscriptionResponse { pub subscription_id: Uuid, pub status: String, pub amount: i64, pub currency: String, pub interval: String, pub current_period_end: String }

#[async_trait]
impl SubscriptionService for SubscriptionServiceImpl {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<SubscriptionResponse, PlatformError> {
        let interval = BillingInterval::from_str(&cmd.interval)
            .map_err(|e| PlatformError::Validation(platform_error::ValidationError::MissingField(e.to_string())))?;
        let sub = Subscription::new(cmd.operator_id, cmd.customer_id, cmd.amount, interval);
        Ok(sub_to_response(&sub))
    }
    async fn get_subscription(&self, id: Uuid) -> Result<SubscriptionResponse, PlatformError> { Err(PlatformError::NotFound { resource: "Subscription".into(), id }) }
    async fn pause_subscription(&self, _id: Uuid) -> Result<(), PlatformError> { Ok(()) }
    async fn resume_subscription(&self, _id: Uuid) -> Result<(), PlatformError> { Ok(()) }
    async fn cancel_subscription(&self, _id: Uuid) -> Result<(), PlatformError> { Ok(()) }
}

fn sub_to_response(s: &Subscription) -> SubscriptionResponse { SubscriptionResponse { subscription_id: s.subscription_id, status: s.status.as_str().to_string(), amount: s.amount.amount_minor_units, currency: s.amount.currency.0.clone(), interval: s.interval.as_str().to_string(), current_period_end: s.current_period_end.to_rfc3339() } }
