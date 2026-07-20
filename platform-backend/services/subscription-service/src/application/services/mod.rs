use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::application::commands::*;
use crate::domain::aggregates::Subscription;
use crate::domain::rules::SubscriptionRepository;
use crate::domain::value_objects::SubscriptionInterval;
use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

pub struct SubscriptionServiceImpl {
    repo: Box<dyn SubscriptionRepository>,
    db: DatabaseConnection,
}

impl SubscriptionServiceImpl {
    pub fn new(repo: Box<dyn SubscriptionRepository>, db: DatabaseConnection) -> Self {
        Self { repo, db }
    }
}

#[async_trait]
pub trait SubscriptionService: Send + Sync {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<Uuid, PlatformError>;
    async fn cancel_subscription(&self, cmd: CancelSubscriptionCommand) -> Result<(), PlatformError>;
    async fn get_subscription(&self, id: Uuid) -> Result<Subscription, PlatformError>;
    async fn list_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, PlatformError>;
}

#[async_trait]
impl SubscriptionService for SubscriptionServiceImpl {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<Uuid, PlatformError> {
        let amount = Money {
            amount_minor_units: cmd.amount_minor_units,
            currency: CurrencyCode::new(&cmd.currency)
                .map_err(|_| PlatformError::Validation(platform_error::ValidationError::InvalidCurrencyCode))?,
        };

        let mut sub = Subscription::new(
            cmd.operator_id,
            cmd.customer_id,
            amount,
            SubscriptionInterval::from_str(&cmd.interval),
            cmd.interval_count.unwrap_or(1),
            cmd.trial_period_days.unwrap_or(0),
        );

        sub.payment_method_token_id = cmd.payment_method_token_id;
        self.repo.save(&sub).await?;
        Ok(sub.subscription_id)
    }

    async fn cancel_subscription(&self, cmd: CancelSubscriptionCommand) -> Result<(), PlatformError> {
        let mut sub = self.repo.find_by_id(cmd.subscription_id).await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "subscription".to_string(),
                id: cmd.subscription_id,
            })?;

        sub.cancel(&cmd.reason);
        self.repo.save(&sub).await?;
        Ok(())
    }

    async fn get_subscription(&self, id: Uuid) -> Result<Subscription, PlatformError> {
        self.repo.find_by_id(id).await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "subscription".to_string(),
                id,
            })
    }

    async fn list_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, PlatformError> {
        self.repo.list_by_customer(customer_id).await
    }
}
