use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::application::commands::*;
use crate::domain::aggregates::PaymentLink;
use crate::domain::rules::{PaginationParams, PaymentLinkFilter, PaymentLinkRepository};
use platform_error::PlatformError;
use platform_middleware::{evaluate_policy, AbacContext};
use shared_types::{CurrencyCode, Money};

pub struct PaymentLinkServiceImpl {
    repo: Box<dyn PaymentLinkRepository>,
    db: DatabaseConnection,
}

impl PaymentLinkServiceImpl {
    pub fn new(repo: Box<dyn PaymentLinkRepository>, db: DatabaseConnection) -> Self {
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
pub trait PaymentLinkService: Send + Sync {
    async fn create(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLink, PlatformError>;
    async fn use_link(&self, cmd: UsePaymentLinkCommand) -> Result<PaymentLink, PlatformError>;
    async fn deactivate(&self, cmd: DeactivatePaymentLinkCommand) -> Result<(), PlatformError>;
    async fn update_metadata(
        &self,
        cmd: UpdatePaymentLinkMetadataCommand,
    ) -> Result<(), PlatformError>;
    async fn get_by_id(&self, id: Uuid) -> Result<PaymentLink, PlatformError>;
    async fn get_by_token(&self, token: &str) -> Result<PaymentLink, PlatformError>;
    async fn list_links(&self, cmd: ListPaymentLinksCommand) -> Result<Vec<PaymentLink>, PlatformError>;
}

#[async_trait]
impl PaymentLinkService for PaymentLinkServiceImpl {
    async fn create(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLink, PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "create", "payment_link")?;

        let amount = Money {
            amount_minor_units: cmd.amount_minor_units,
            currency: CurrencyCode::new(&cmd.currency)
                .map_err(|_| PlatformError::Validation(
                    platform_error::ValidationError::InvalidCurrencyCode,
                ))?,
        };

        if amount.amount_minor_units <= 0 {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::NegativeAmount,
            ));
        }

        let expires_at = cmd
            .expires_in_hours
            .map(|h| chrono::Utc::now() + chrono::Duration::hours(h));

        let mut link = PaymentLink::new(
            cmd.operator_id,
            cmd.description,
            cmd.merchant_name,
            amount,
            cmd.max_uses,
            expires_at,
        );

        if let Some(metadata) = cmd.metadata {
            link.update_metadata(metadata);
        }

        self.repo.save(&link).await?;
        Ok(link)
    }

    async fn use_link(&self, cmd: UsePaymentLinkCommand) -> Result<PaymentLink, PlatformError> {
        let mut link = self
            .repo
            .find_by_token(&cmd.public_token)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "payment_link".into(),
                id: Uuid::nil(),
            })?;

        link.use_link().map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: "active".into(),
                command: e.to_string(),
            })
        })?;

        self.repo.save(&link).await?;
        Ok(link)
    }

    async fn deactivate(&self, cmd: DeactivatePaymentLinkCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "payment_link")?;

        let mut link = self
            .repo
            .find_by_id(cmd.link_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "payment_link".into(),
                id: cmd.link_id,
            })?;

        link.deactivate().map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: link.status.as_str().into(),
                command: e.to_string(),
            })
        })?;

        self.repo.save(&link).await
    }

    async fn update_metadata(
        &self,
        cmd: UpdatePaymentLinkMetadataCommand,
    ) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "payment_link")?;

        let mut link = self
            .repo
            .find_by_id(cmd.link_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "payment_link".into(),
                id: cmd.link_id,
            })?;

        link.update_metadata(cmd.metadata);
        self.repo.save(&link).await
    }

    async fn get_by_id(&self, id: Uuid) -> Result<PaymentLink, PlatformError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "payment_link".into(),
                id,
            })
    }

    async fn get_by_token(&self, token: &str) -> Result<PaymentLink, PlatformError> {
        self.repo
            .find_by_token(token)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "payment_link".into(),
                id: Uuid::nil(),
            })
    }

    async fn list_links(
        &self,
        cmd: ListPaymentLinksCommand,
    ) -> Result<Vec<PaymentLink>, PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "read", "payment_link")?;

        let filter = PaymentLinkFilter {
            status: cmd.status,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::aggregates::PaymentLink;
    use crate::domain::rules::*;
    use crate::domain::value_objects::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct InMemoryPaymentLinkRepository {
        store: Mutex<HashMap<Uuid, PaymentLink>>,
        token_index: Mutex<HashMap<String, Uuid>>,
    }

    impl InMemoryPaymentLinkRepository {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
                token_index: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl PaymentLinkRepository for InMemoryPaymentLinkRepository {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<PaymentLink>, PlatformError> {
            let store = self.store.lock().unwrap();
            Ok(store.get(&id).cloned())
        }

        async fn find_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PlatformError> {
            let index = self.token_index.lock().unwrap();
            let store = self.store.lock().unwrap();
            Ok(index.get(token).and_then(|id| store.get(id)).cloned())
        }

        async fn save(&self, link: &PaymentLink) -> Result<(), PlatformError> {
            let mut store = self.store.lock().unwrap();
            let mut index = self.token_index.lock().unwrap();
            index.insert(link.public_token.clone(), link.link_id);
            store.insert(link.link_id, link.clone());
            Ok(())
        }

        async fn list_by_operator(
            &self,
            operator_id: Uuid,
            filter: &PaymentLinkFilter,
            pagination: &PaginationParams,
        ) -> Result<Vec<PaymentLink>, PlatformError> {
            let store = self.store.lock().unwrap();
            let mut results: Vec<PaymentLink> = store
                .values()
                .filter(|l| l.operator_id == operator_id)
                .filter(|l| {
                    filter
                        .status
                        .as_ref()
                        .map(|st| l.status.as_str() == st.as_str())
                        .unwrap_or(true)
                })
                .filter(|l| {
                    filter
                        .min_amount
                        .map(|min| l.amount.amount_minor_units >= min)
                        .unwrap_or(true)
                })
                .filter(|l| {
                    filter
                        .max_amount
                        .map(|max| l.amount.amount_minor_units <= max)
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            // Apply pagination
            let offset = pagination.offset.max(0) as usize;
            let limit = pagination.limit.max(1).min(100) as usize;
            let results: Vec<PaymentLink> = results.into_iter().skip(offset).take(limit).collect();
            Ok(results)
        }

        async fn find_expired_links(
            &self,
            _before: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<PaymentLink>, PlatformError> {
            Ok(Vec::new())
        }
    }

    fn test_service() -> PaymentLinkServiceImpl {
        let repo = Box::new(InMemoryPaymentLinkRepository::new());
        let db = sea_orm::DatabaseConnection::default();
        PaymentLinkServiceImpl::new(repo, db)
    }

    fn admin_auth() -> (Uuid, String) {
        (Uuid::now_v7(), "platform_admin".to_string())
    }

    #[tokio::test]
    async fn test_create_payment_link() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreatePaymentLinkCommand {
            operator_id: Uuid::now_v7(),
            description: "Test payment link".to_string(),
            merchant_name: "Test Merchant".to_string(),
            amount_minor_units: 5000,
            currency: "AED".to_string(),
            max_uses: Some(10),
            expires_in_hours: Some(24),
            metadata: Some(serde_json::json!({"campaign": "summer"})),
            principal_id: principal,
            role,
        };

        let link = service.create(cmd).await.unwrap();
        assert_eq!(link.amount.amount_minor_units, 5000);
        assert_eq!(link.status, PaymentLinkStatus::Active);
        assert_eq!(link.max_uses, Some(10));
        assert!(link.expires_at.is_some());
        assert_eq!(
            link.metadata,
            Some(serde_json::json!({"campaign": "summer"}))
        );
    }

    #[tokio::test]
    async fn test_create_payment_link_negative_amount() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreatePaymentLinkCommand {
            operator_id: Uuid::now_v7(),
            description: "Test".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: -100,
            currency: "AED".to_string(),
            max_uses: None,
            expires_in_hours: None,
            metadata: None,
            principal_id: principal,
            role,
        };

        let result = service.create(cmd).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_use_payment_link() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreatePaymentLinkCommand {
            operator_id: Uuid::now_v7(),
            description: "Test".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: 1000,
            currency: "AED".to_string(),
            max_uses: Some(3),
            expires_in_hours: None,
            metadata: None,
            principal_id: principal,
            role,
        };

        let link = service.create(cmd).await.unwrap();
        assert_eq!(link.current_uses, 0);

        let use_cmd = UsePaymentLinkCommand {
            public_token: link.public_token.clone(),
        };
        let updated = service.use_link(use_cmd).await.unwrap();
        assert_eq!(updated.current_uses, 1);
        assert_eq!(updated.status, PaymentLinkStatus::Active);

        let use_cmd = UsePaymentLinkCommand {
            public_token: link.public_token.clone(),
        };
        let updated = service.use_link(use_cmd).await.unwrap();
        assert_eq!(updated.current_uses, 2);

        let use_cmd = UsePaymentLinkCommand {
            public_token: link.public_token.clone(),
        };
        let updated = service.use_link(use_cmd).await.unwrap();
        assert_eq!(updated.current_uses, 3);
        assert_eq!(updated.status, PaymentLinkStatus::UsedUp);

        let use_cmd = UsePaymentLinkCommand {
            public_token: link.public_token,
        };
        let result = service.use_link(use_cmd).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_use_payment_link_not_found() {
        let service = test_service();
        let use_cmd = UsePaymentLinkCommand {
            public_token: "nonexistent".to_string(),
        };
        let result = service.use_link(use_cmd).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_deactivate_payment_link() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreatePaymentLinkCommand {
            operator_id: Uuid::now_v7(),
            description: "Test".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: 1000,
            currency: "AED".to_string(),
            max_uses: None,
            expires_in_hours: None,
            metadata: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let link = service.create(cmd).await.unwrap();

        let deactivate_cmd = DeactivatePaymentLinkCommand {
            link_id: link.link_id,
            principal_id: principal,
            role,
        };

        service.deactivate(deactivate_cmd).await.unwrap();

        let updated = service.get_by_id(link.link_id).await.unwrap();
        assert_eq!(updated.status, PaymentLinkStatus::Deactivated);
        assert!(!updated.is_valid());
    }

    #[tokio::test]
    async fn test_deactivate_already_deactivated() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreatePaymentLinkCommand {
            operator_id: Uuid::now_v7(),
            description: "Test".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: 1000,
            currency: "AED".to_string(),
            max_uses: None,
            expires_in_hours: None,
            metadata: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let link = service.create(cmd).await.unwrap();

        let deactivate_cmd = DeactivatePaymentLinkCommand {
            link_id: link.link_id,
            principal_id: principal.clone(),
            role: role.clone(),
        };
        service.deactivate(deactivate_cmd).await.unwrap();

        let deactivate_cmd = DeactivatePaymentLinkCommand {
            link_id: link.link_id,
            principal_id: principal,
            role,
        };
        let result = service.deactivate(deactivate_cmd).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_metadata() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreatePaymentLinkCommand {
            operator_id: Uuid::now_v7(),
            description: "Test".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: 1000,
            currency: "AED".to_string(),
            max_uses: None,
            expires_in_hours: None,
            metadata: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let link = service.create(cmd).await.unwrap();
        assert!(link.metadata.is_none());

        let update_cmd = UpdatePaymentLinkMetadataCommand {
            link_id: link.link_id,
            metadata: serde_json::json!({"key": "value"}),
            principal_id: principal,
            role,
        };

        service.update_metadata(update_cmd).await.unwrap();

        let updated = service.get_by_id(link.link_id).await.unwrap();
        assert_eq!(
            updated.metadata,
            Some(serde_json::json!({"key": "value"}))
        );
    }

    #[tokio::test]
    async fn test_get_by_token() {
        let service = test_service();
        let (principal, role) = admin_auth();

        let cmd = CreatePaymentLinkCommand {
            operator_id: Uuid::now_v7(),
            description: "Test".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: 1000,
            currency: "AED".to_string(),
            max_uses: None,
            expires_in_hours: None,
            metadata: None,
            principal_id: principal,
            role,
        };

        let link = service.create(cmd).await.unwrap();
        let found = service.get_by_token(&link.public_token).await.unwrap();
        assert_eq!(found.link_id, link.link_id);
    }

    #[tokio::test]
    async fn test_list_links() {
        let service = test_service();
        let (principal, role) = admin_auth();
        let operator_id = Uuid::now_v7();

        for i in 0..5 {
            let cmd = CreatePaymentLinkCommand {
                operator_id,
                description: format!("Link {}", i),
                merchant_name: "Merchant".to_string(),
                amount_minor_units: (i + 1) * 1000,
                currency: "AED".to_string(),
                max_uses: None,
                expires_in_hours: None,
                metadata: None,
                principal_id: principal.clone(),
                role: role.clone(),
            };
            service.create(cmd).await.unwrap();
        }

        let list_cmd = ListPaymentLinksCommand {
            operator_id,
            status: None,
            min_amount: None,
            max_amount: None,
            limit: Some(3),
            offset: Some(0),
            principal_id: principal.clone(),
            role: role.clone(),
        };

        let links = service.list_links(list_cmd).await.unwrap();
        assert_eq!(links.len(), 3);
    }

    #[tokio::test]
    async fn test_list_links_with_status_filter() {
        let service = test_service();
        let (principal, role) = admin_auth();
        let operator_id = Uuid::now_v7();

        let cmd = CreatePaymentLinkCommand {
            operator_id,
            description: "Active".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: 1000,
            currency: "AED".to_string(),
            max_uses: None,
            expires_in_hours: None,
            metadata: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };
        let link = service.create(cmd).await.unwrap();

        let deactivate_cmd = DeactivatePaymentLinkCommand {
            link_id: link.link_id,
            principal_id: principal.clone(),
            role: role.clone(),
        };
        service.deactivate(deactivate_cmd).await.unwrap();

        let cmd = CreatePaymentLinkCommand {
            operator_id,
            description: "Another".to_string(),
            merchant_name: "Merchant".to_string(),
            amount_minor_units: 2000,
            currency: "AED".to_string(),
            max_uses: None,
            expires_in_hours: None,
            metadata: None,
            principal_id: principal.clone(),
            role: role.clone(),
        };
        service.create(cmd).await.unwrap();

        let list_cmd = ListPaymentLinksCommand {
            operator_id,
            status: Some("active".to_string()),
            min_amount: None,
            max_amount: None,
            limit: Some(10),
            offset: Some(0),
            principal_id: principal,
            role,
        };

        let links = service.list_links(list_cmd).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].status, PaymentLinkStatus::Active);
    }
}
