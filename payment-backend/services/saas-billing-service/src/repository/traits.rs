//! Repository traits for SaaS Billing service.

use async_trait::async_trait;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::*;

/// Repository trait for SaaS Plan operations.
#[async_trait]
pub trait SaasPlanRepository: Send + Sync {
    async fn find_by_id(&self, plan_id: Uuid) -> Result<Option<SaasPlan>, SaaSbillingError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<SaasPlan>, SaaSbillingError>;
    async fn list_active(&self) -> Result<Vec<SaasPlan>, SaaSbillingError>;
    async fn list_all(&self) -> Result<Vec<SaasPlan>, SaaSbillingError>;
}

/// Repository trait for Tenant Subscription operations.
#[async_trait]
pub trait TenantSubscriptionRepository: Send + Sync {
    async fn find_by_id(&self, subscription_id: Uuid) -> Result<Option<TenantSubscription>, SaaSbillingError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Option<TenantSubscription>, SaaSbillingError>;
    async fn save(&self, subscription: &mut TenantSubscription) -> Result<(), SaaSbillingError>;
    async fn update(&self, subscription: &TenantSubscription) -> Result<(), SaaSbillingError>;
    async fn count_by_plan(&self, plan_id: Uuid) -> Result<i64, SaaSbillingError>;
}

/// Repository trait for Tenant Usage operations.
#[async_trait]
pub trait TenantUsageRepository: Send + Sync {
    async fn find_by_operator_and_period(
        &self,
        operator_id: Uuid,
        period_start: DateTime<Utc>,
    ) -> Result<Option<TenantUsage>, SaaSbillingError>;
    async fn save(&self, usage: &mut TenantUsage) -> Result<(), SaaSbillingError>;
    async fn update(&self, usage: &TenantUsage) -> Result<(), SaaSbillingError>;
    async fn get_current_period_usage(&self, operator_id: Uuid) -> Result<Option<TenantUsage>, SaaSbillingError>;
}

/// Repository trait for SaaS Invoice operations.
#[async_trait]
pub trait SaasInvoiceRepository: Send + Sync {
    async fn find_by_id(&self, invoice_id: Uuid) -> Result<Option<SaasInvoice>, SaaSbillingError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<SaasInvoice>, SaaSbillingError>;
    async fn find_by_subscription(&self, subscription_id: Uuid) -> Result<Vec<SaasInvoice>, SaaSbillingError>;
    async fn save(&self, invoice: &mut SaasInvoice) -> Result<(), SaaSbillingError>;
    async fn update(&self, invoice: &SaasInvoice) -> Result<(), SaaSbillingError>;
    async fn find_open_invoice_for_period(
        &self,
        subscription_id: Uuid,
        period_start: DateTime<Utc>,
    ) -> Result<Option<SaasInvoice>, SaaSbillingError>;
}

/// Repository trait for Team Member operations.
#[async_trait]
pub trait TeamMemberRepository: Send + Sync {
    async fn find_by_id(&self, membership_id: Uuid) -> Result<Option<TenantTeamMember>, SaaSbillingError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<TenantTeamMember>, SaaSbillingError>;
    async fn find_by_principal(&self, principal_id: Uuid) -> Result<Vec<TenantTeamMember>, SaaSbillingError>;
    async fn save(&self, member: &mut TenantTeamMember) -> Result<(), SaaSbillingError>;
    async fn update(&self, member: &TenantTeamMember) -> Result<(), SaaSbillingError>;
    async fn count_active_by_operator(&self, operator_id: Uuid) -> Result<i64, SaaSbillingError>;
}

/// Repository trait for Audit Log operations.
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn save(&self, log: &AuditLog) -> Result<(), SaaSbillingError>;
    async fn find_by_operator(
        &self,
        operator_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, SaaSbillingError>;
    async fn find_by_action(
        &self,
        operator_id: Uuid,
        action: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, SaaSbillingError>;
}
