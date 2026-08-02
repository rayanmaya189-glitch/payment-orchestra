//! Query handler implementation for SaaS Billing service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::*;
use crate::commands::types::*;

/// Query handler trait for SaaS Billing.
#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_plans(&self) -> Result<Vec<SaasPlan>, SaaSbillingError>;
    async fn get_plan(&self, plan_id: Uuid) -> Result<Option<SaasPlan>, SaaSbillingError>;
    async fn get_plan_by_slug(&self, slug: &str) -> Result<Option<SaasPlan>, SaaSbillingError>;
    
    async fn get_subscription(&self, query: GetSubscriptionQuery) -> Result<Option<TenantSubscription>, SaaSbillingError>;
    
    async fn get_current_usage(&self, query: GetCurrentUsageQuery) -> Result<Option<TenantUsage>, SaaSbillingError>;
    
    async fn get_invoices(&self, query: GetInvoicesQuery) -> Result<Vec<SaasInvoice>, SaaSbillingError>;
    async fn get_invoice(&self, invoice_id: Uuid, operator_id: Uuid) -> Result<Option<SaasInvoice>, SaaSbillingError>;
    
    async fn get_team_members(&self, query: GetTeamMembersQuery) -> Result<Vec<TenantTeamMember>, SaaSbillingError>;
    
    async fn get_audit_logs(&self, query: GetAuditLogsQuery) -> Result<Vec<AuditLog>, SaaSbillingError>;
}

/// SaaS Billing query handler implementation.
pub struct SaasBillingQueryHandler<P, S, U, I, T, A> {
    plan_repo: P,
    subscription_repo: S,
    usage_repo: U,
    invoice_repo: I,
    team_repo: T,
    audit_repo: A,
}

impl<P, S, U, I, T, A> SaasBillingQueryHandler<P, S, U, I, T, A> {
    pub fn new(
        plan_repo: P,
        subscription_repo: S,
        usage_repo: U,
        invoice_repo: I,
        team_repo: T,
        audit_repo: A,
    ) -> Self {
        Self {
            plan_repo,
            subscription_repo,
            usage_repo,
            invoice_repo,
            team_repo,
            audit_repo,
        }
    }
}

#[async_trait]
impl<P, S, U, I, T, A> QueryHandler for SaasBillingQueryHandler<P, S, U, I, T, A>
where
    P: SaasPlanRepository,
    S: TenantSubscriptionRepository,
    U: TenantUsageRepository,
    I: SaasInvoiceRepository,
    T: TeamMemberRepository,
    A: AuditLogRepository,
{
    async fn get_plans(&self) -> Result<Vec<SaasPlan>, SaaSbillingError> {
        self.plan_repo.list_active().await
    }

    async fn get_plan(&self, plan_id: Uuid) -> Result<Option<SaasPlan>, SaaSbillingError> {
        self.plan_repo.find_by_id(plan_id).await
    }

    async fn get_plan_by_slug(&self, slug: &str) -> Result<Option<SaasPlan>, SaaSbillingError> {
        self.plan_repo.find_by_slug(slug).await
    }

    async fn get_subscription(&self, query: GetSubscriptionQuery) -> Result<Option<TenantSubscription>, SaaSbillingError> {
        self.subscription_repo.find_by_operator(query.operator_id).await
    }

    async fn get_current_usage(&self, query: GetCurrentUsageQuery) -> Result<Option<TenantUsage>, SaaSbillingError> {
        self.usage_repo.get_current_period_usage(query.operator_id).await
    }

    async fn get_invoices(&self, query: GetInvoicesQuery) -> Result<Vec<SaasInvoice>, SaaSbillingError> {
        let mut invoices = self.invoice_repo.find_by_operator(query.operator_id).await?;
        
        // Sort by created_at descending
        invoices.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Apply pagination
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(50) as usize;
        
        Ok(invoices.into_iter().skip(offset).take(limit).collect())
    }

    async fn get_invoice(&self, invoice_id: Uuid, operator_id: Uuid) -> Result<Option<SaasInvoice>, SaaSbillingError> {
        let invoice = self.invoice_repo.find_by_id(invoice_id).await?;
        
        // Verify ownership
        if let Some(ref inv) = invoice {
            if inv.operator_id != operator_id {
                return Ok(None);
            }
        }
        
        Ok(invoice)
    }

    async fn get_team_members(&self, query: GetTeamMembersQuery) -> Result<Vec<TenantTeamMember>, SaaSbillingError> {
        self.team_repo.find_by_operator(query.operator_id).await
    }

    async fn get_audit_logs(&self, query: GetAuditLogsQuery) -> Result<Vec<AuditLog>, SaaSbillingError> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        
        if let Some(action) = query.action_filter {
            self.audit_repo.find_by_action(query.operator_id, &action, limit).await
        } else {
            self.audit_repo.find_by_operator(query.operator_id, limit, offset).await
        }
    }
}
