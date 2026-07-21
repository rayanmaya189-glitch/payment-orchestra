use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::application::commands::*;
use crate::domain::aggregates::Subscription;
use crate::domain::rules::{
    PaginationParams, SubscriptionFilter, SubscriptionRepository,
};
use crate::domain::value_objects::{DunningConfig, SubscriptionInterval};
use platform_error::PlatformError;
use platform_middleware::{evaluate_policy, AbacContext};
use shared_types::{CurrencyCode, Money};

pub struct SubscriptionServiceImpl {
    repo: Box<dyn SubscriptionRepository>,
    db: DatabaseConnection,
}

impl SubscriptionServiceImpl {
    pub fn new(repo: Box<dyn SubscriptionRepository>, db: DatabaseConnection) -> Self {
        Self { repo, db }
    }

    fn check_abac(
        principal_id: Uuid,
        role: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), PlatformError> {
        evaluate_policy(&AbacContext {
            principal_id,
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        })
    }
}

#[async_trait]
pub trait SubscriptionService: Send + Sync {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<Uuid, PlatformError>;
    async fn cancel_subscription(&self, cmd: CancelSubscriptionCommand) -> Result<(), PlatformError>;
    async fn charge_subscription(&self, cmd: ChargeSubscriptionCommand) -> Result<(), PlatformError>;
    async fn reactivate_subscription(&self, cmd: ReactivateSubscriptionCommand) -> Result<(), PlatformError>;
    async fn update_payment_method(&self, cmd: UpdatePaymentMethodCommand) -> Result<(), PlatformError>;
    async fn get_subscription(&self, id: Uuid) -> Result<Subscription, PlatformError>;
    async fn list_subscriptions(&self, cmd: ListSubscriptionsCommand) -> Result<Vec<Subscription>, PlatformError>;
    async fn list_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, PlatformError>;
    async fn find_due_subscriptions(&self) -> Result<Vec<Subscription>, PlatformError>;
}

#[async_trait]
impl SubscriptionService for SubscriptionServiceImpl {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<Uuid, PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "create", "subscription")?;

        let amount = Money {
            amount_minor_units: cmd.amount_minor_units,
            currency: CurrencyCode::new(&cmd.currency)
                .map_err(|_| PlatformError::Validation(platform_error::ValidationError::InvalidCurrencyCode))?,
        };

        let interval = SubscriptionInterval::from_str(&cmd.interval);
        let interval_count = cmd.interval_count.unwrap_or(1);
        let trial_period_days = cmd.trial_period_days.unwrap_or(0);

        let mut sub = Subscription::new(
            cmd.operator_id,
            cmd.customer_id,
            amount,
            interval,
            interval_count,
            trial_period_days,
        );

        sub.payment_method_token_id = cmd.payment_method_token_id;

        // Apply dunning profile if specified
        if let Some(profile) = &cmd.dunning_profile {
            let dunning = DunningConfig::from_profile(profile);
            sub.max_retries = dunning.max_retries;
        }

        self.repo.save(&sub).await?;
        Ok(sub.subscription_id)
    }

    async fn cancel_subscription(&self, cmd: CancelSubscriptionCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "subscription")?;

        let mut sub = self
            .repo
            .find_by_id(cmd.subscription_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "subscription".into(),
                id: cmd.subscription_id,
            })?;

        sub.cancel(&cmd.reason);
        self.repo.save(&sub).await
    }

    async fn charge_subscription(&self, cmd: ChargeSubscriptionCommand) -> Result<(), PlatformError> {
        let mut sub = self
            .repo
            .find_by_id(cmd.subscription_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "subscription".into(),
                id: cmd.subscription_id,
            })?;

        // Validate the subscription can be charged
        sub.validate_for_charge()
            .map_err(|e| PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: sub.status.as_str().to_string(),
                command: e.to_string(),
            }))?;

        // Simulate payment processing via billing integration
        // In production, this would call the billing/payment-intent-service
        let payment_succeeded = true; // Stub: always succeeds

        if payment_succeeded {
            sub.record_payment_success(cmd.payment_intent_id);
        } else {
            let dunning = DunningConfig::standard();
            sub.record_payment_failure(cmd.payment_intent_id, "billing_declined");
            if sub.status == crate::domain::value_objects::SubscriptionStatus::PastDue {
                sub.mark_past_due(&dunning);
            }
        }

        self.repo.save(&sub).await
    }

    async fn reactivate_subscription(&self, cmd: ReactivateSubscriptionCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "subscription")?;

        let mut sub = self
            .repo
            .find_by_id(cmd.subscription_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "subscription".into(),
                id: cmd.subscription_id,
            })?;

        sub.reactivate()
            .map_err(|e| PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: sub.status.as_str().to_string(),
                command: e.to_string(),
            }))?;

        self.repo.save(&sub).await
    }

    async fn update_payment_method(&self, cmd: UpdatePaymentMethodCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "subscription")?;

        let mut sub = self
            .repo
            .find_by_id(cmd.subscription_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "subscription".into(),
                id: cmd.subscription_id,
            })?;

        sub.update_payment_method(cmd.new_payment_method_token);
        self.repo.save(&sub).await
    }

    async fn get_subscription(&self, id: Uuid) -> Result<Subscription, PlatformError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "subscription".into(),
                id,
            })
    }

    async fn list_subscriptions(&self, cmd: ListSubscriptionsCommand) -> Result<Vec<Subscription>, PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "read", "subscription")?;

        let filter = SubscriptionFilter {
            status: cmd.status,
            customer_id: cmd.customer_id,
            min_amount: cmd.min_amount,
            max_amount: cmd.max_amount,
        };

        let pagination = PaginationParams {
            limit: cmd.limit.unwrap_or(20).min(100),
            offset: cmd.offset.unwrap_or(0),
        };

        self.repo
            .list_by_operator(cmd.operator_id, &filter, &pagination)
            .await
    }

    async fn list_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, PlatformError> {
        self.repo.list_by_customer(customer_id).await
    }

    async fn find_due_subscriptions(&self) -> Result<Vec<Subscription>, PlatformError> {
        self.repo.find_due_subscriptions().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::aggregates::Subscription;
    use crate::domain::rules::*;
    use crate::domain::value_objects::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    /// In-memory repository for testing.
    struct InMemorySubscriptionRepository {
        store: Mutex<HashMap<Uuid, Subscription>>,
    }

    impl InMemorySubscriptionRepository {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl SubscriptionRepository for InMemorySubscriptionRepository {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<Subscription>, PlatformError> {
            let store = self.store.lock().unwrap();
            Ok(store.get(&id).cloned())
        }

        async fn save(&self, sub: &Subscription) -> Result<(), PlatformError> {
            let mut store = self.store.lock().unwrap();
            store.insert(sub.subscription_id, sub.clone());
            Ok(())
        }

        async fn list_by_customer(
            &self,
            customer_id: Uuid,
        ) -> Result<Vec<Subscription>, PlatformError> {
            let store = self.store.lock().unwrap();
            Ok(store
                .values()
                .filter(|s| s.customer_id == customer_id)
                .cloned()
                .collect())
        }

        async fn list_by_operator(
            &self,
            operator_id: Uuid,
            filter: &SubscriptionFilter,
            pagination: &PaginationParams,
        ) -> Result<Vec<Subscription>, PlatformError> {
            let store = self.store.lock().unwrap();
            let mut results: Vec<Subscription> = store
                .values()
                .filter(|s| s.operator_id == operator_id)
                .filter(|s| {
                    filter
                        .status
                        .as_ref()
                        .map(|st| s.status.as_str() == st.as_str())
                        .unwrap_or(true)
                })
                .filter(|s| {
                    filter
                        .min_amount
                        .map(|min| s.amount.amount_minor_units >= min)
                        .unwrap_or(true)
                })
                .filter(|s| {
                    filter
                        .max_amount
                        .map(|max| s.amount.amount_minor_units <= max)
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            // Apply pagination
            let offset = pagination.offset.max(0) as usize;
            let limit = pagination.limit.max(1).min(100) as usize;
            let results: Vec<Subscription> = results.into_iter().skip(offset).take(limit).collect();
            Ok(results)
        }

        async fn find_due_subscriptions(&self) -> Result<Vec<Subscription>, PlatformError> {
            let store = self.store.lock().unwrap();
            let now = chrono::Utc::now();
            Ok(store
                .values()
                .filter(|s| {
                    matches!(
                        s.status,
                        SubscriptionStatus::Active
                            | SubscriptionStatus::PastDue
                            | SubscriptionStatus::Trialing
                    ) && s.current_period_end <= now
                })
                .cloned()
                .collect())
        }

        async fn find_canceled_since(
            &self,
            since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Subscription>, PlatformError> {
            let store = self.store.lock().unwrap();
            Ok(store
                .values()
                .filter(|s| {
                    s.status == SubscriptionStatus::Canceled
                        && s.canceled_at.map(|dt| dt >= since).unwrap_or(false)
                })
                .cloned()
                .collect())
        }
    }

    fn aed(amount: i64) -> Money {
        Money {
            amount_minor_units: amount,
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
        }
    }

    fn test_service() -> SubscriptionServiceImpl {
        let repo = Box::new(InMemorySubscriptionRepository::new());
        let db = sea_orm::DatabaseConnection::default();
        SubscriptionServiceImpl::new(repo, db)
    }

    fn admin_auth() -> (Uuid, String) {
        (Uuid::now_v7(), "platform_admin".to_string())
    }

    #[tokio::test]
    async fn test_create_subscription() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 5000,
            currency: "AED".to_string(),
            interval: "monthly".to_string(),
            interval_count: Some(1),
            trial_period_days: None,
            payment_method_token_id: Some("tok_123".to_string()),
            dunning_profile: None,
            principal_id: principal,
            role,
        };

        let id = service.create_subscription(cmd).await.unwrap();
        let sub = service.get_subscription(id).await.unwrap();

        assert_eq!(sub.amount.amount_minor_units, 5000);
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert_eq!(
            sub.payment_method_token_id,
            Some("tok_123".to_string())
        );
    }

    #[tokio::test]
    async fn test_create_subscription_with_trial() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 10000,
            currency: "USD".to_string(),
            interval: "yearly".to_string(),
            interval_count: Some(1),
            trial_period_days: Some(30),
            payment_method_token_id: None,
            dunning_profile: None,
            principal_id: principal,
            role,
        };

        let id = service.create_subscription(cmd).await.unwrap();
        let sub = service.get_subscription(id).await.unwrap();

        assert_eq!(sub.status, SubscriptionStatus::Trialing);
        assert_eq!(sub.trial_period_days, 30);
    }

    #[tokio::test]
    async fn test_create_subscription_with_dunning_profile() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 2000,
            currency: "AED".to_string(),
            interval: "monthly".to_string(),
            interval_count: Some(1),
            trial_period_days: None,
            payment_method_token_id: Some("tok_456".to_string()),
            dunning_profile: Some("aggressive".to_string()),
            principal_id: principal,
            role,
        };

        let id = service.create_subscription(cmd).await.unwrap();
        let sub = service.get_subscription(id).await.unwrap();

        assert_eq!(sub.max_retries, 5); // aggressive profile
    }

    #[tokio::test]
    async fn test_cancel_subscription() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 3000,
            currency: "AED".to_string(),
            interval: "monthly".to_string(),
            interval_count: Some(1),
            trial_period_days: None,
            payment_method_token_id: None,
            dunning_profile: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let id = service.create_subscription(cmd).await.unwrap();

        let cancel_cmd = CancelSubscriptionCommand {
            subscription_id: id,
            reason: "customer_request".to_string(),
            principal_id: principal,
            role,
        };

        service.cancel_subscription(cancel_cmd).await.unwrap();
        let sub = service.get_subscription(id).await.unwrap();

        assert_eq!(sub.status, SubscriptionStatus::Canceled);
        assert!(sub.canceled_at.is_some());
    }

    #[tokio::test]
    async fn test_charge_subscription_success() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 5000,
            currency: "AED".to_string(),
            interval: "monthly".to_string(),
            interval_count: Some(1),
            trial_period_days: None,
            payment_method_token_id: Some("tok_123".to_string()),
            dunning_profile: None,
            principal_id: principal,
            role,
        };

        let id = service.create_subscription(cmd).await.unwrap();

        let charge_cmd = ChargeSubscriptionCommand {
            subscription_id: id,
            payment_intent_id: Some(Uuid::now_v7()),
        };

        service.charge_subscription(charge_cmd).await.unwrap();
        let sub = service.get_subscription(id).await.unwrap();

        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert_eq!(sub.retry_count, 0);
    }

    #[tokio::test]
    async fn test_charge_subscription_no_payment_method() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 5000,
            currency: "AED".to_string(),
            interval: "monthly".to_string(),
            interval_count: Some(1),
            trial_period_days: None,
            payment_method_token_id: None,
            dunning_profile: None,
            principal_id: principal,
            role,
        };

        let id = service.create_subscription(cmd).await.unwrap();

        let charge_cmd = ChargeSubscriptionCommand {
            subscription_id: id,
            payment_intent_id: None,
        };

        let result = service.charge_subscription(charge_cmd).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reactivate_subscription() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 5000,
            currency: "AED".to_string(),
            interval: "monthly".to_string(),
            interval_count: Some(1),
            trial_period_days: None,
            payment_method_token_id: Some("tok_123".to_string()),
            dunning_profile: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let id = service.create_subscription(cmd).await.unwrap();

        // Cancel first
        let cancel_cmd = CancelSubscriptionCommand {
            subscription_id: id,
            reason: "test".to_string(),
            principal_id: principal.clone(),
            role: role.clone(),
        };
        service.cancel_subscription(cancel_cmd).await.unwrap();

        // Reactivate
        let reactivate_cmd = ReactivateSubscriptionCommand {
            subscription_id: id,
            principal_id: principal,
            role,
        };
        service.reactivate_subscription(reactivate_cmd).await.unwrap();

        let sub = service.get_subscription(id).await.unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert!(sub.canceled_at.is_none());
    }

    #[tokio::test]
    async fn test_update_payment_method() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreateSubscriptionCommand {
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            amount_minor_units: 5000,
            currency: "AED".to_string(),
            interval: "monthly".to_string(),
            interval_count: Some(1),
            trial_period_days: None,
            payment_method_token_id: Some("tok_old".to_string()),
            dunning_profile: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let id = service.create_subscription(cmd).await.unwrap();

        let update_cmd = UpdatePaymentMethodCommand {
            subscription_id: id,
            new_payment_method_token: "tok_new".to_string(),
            principal_id: principal,
            role,
        };

        service.update_payment_method(update_cmd).await.unwrap();
        let sub = service.get_subscription(id).await.unwrap();

        assert_eq!(
            sub.payment_method_token_id,
            Some("tok_new".to_string())
        );
    }

    #[tokio::test]
    async fn test_list_subscriptions() {
        let service = test_service();
        let (principal, role) = admin_auth();
        let operator_id = Uuid::now_v7();

        // Create multiple subscriptions
        for i in 0..5 {
            let cmd = CreateSubscriptionCommand {
                operator_id,
                customer_id: Uuid::now_v7(),
                amount_minor_units: (i + 1) * 1000,
                currency: "AED".to_string(),
                interval: "monthly".to_string(),
                interval_count: Some(1),
                trial_period_days: None,
                payment_method_token_id: None,
                dunning_profile: None,
                principal_id: principal.clone(),
                role: role.clone(),
            };
            service.create_subscription(cmd).await.unwrap();
        }

        let list_cmd = ListSubscriptionsCommand {
            operator_id,
            status: None,
            customer_id: None,
            min_amount: None,
            max_amount: None,
            limit: Some(3),
            offset: Some(0),
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let subs = service.list_subscriptions(list_cmd).await.unwrap();
        assert_eq!(subs.len(), 3);

        // Test with amount filter
        let list_cmd = ListSubscriptionsCommand {
            operator_id,
            status: None,
            customer_id: None,
            min_amount: Some(3000),
            max_amount: None,
            limit: Some(10),
            offset: Some(0),
            principal_id: principal,
            role,
        };

        let subs = service.list_subscriptions(list_cmd).await.unwrap();
        assert_eq!(subs.len(), 3); // 3000, 4000, 5000
    }

    #[tokio::test]
    async fn test_get_subscription_not_found() {
        let service = test_service();
        let result = service.get_subscription(Uuid::now_v7()).await;
        assert!(result.is_err());
    }
}
