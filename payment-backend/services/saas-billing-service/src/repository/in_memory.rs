//! In-memory repository implementation for SaaS Billing service.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{Date, Utc};
use async_trait::async_trait;

use crate::domain::*;
use super::traits::*;

/// In-memory implementation of all SaaS billing repositories.
#[derive(Clone)]
pub struct InMemorySaasBillingRepository {
    pub(super) plans: Arc<RwLock<HashMap<Uuid, SaasPlan>>>,
    pub(super) subscriptions: Arc<RwLock<HashMap<Uuid, TenantSubscription>>>,
    pub(super) subscriptions_by_operator: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    pub(super) usage: Arc<RwLock<HashMap<(Uuid, Date<Utc>), TenantUsage>>>,
    pub(super) invoices: Arc<RwLock<HashMap<Uuid, SaasInvoice>>>,
    pub(super) team_members: Arc<RwLock<HashMap<Uuid, TenantTeamMember>>>,
    pub(super) audit_logs: Arc<RwLock<Vec<AuditLog>>>,
}

impl Default for InMemorySaasBillingRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySaasBillingRepository {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            subscriptions_by_operator: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
            invoices: Arc::new(RwLock::new(HashMap::new())),
            team_members: Arc::new(RwLock::new(HashMap::new())),
            audit_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Seed with test data.
    pub async fn seed_test_data(&self) {
        let mut plans = self.plans.write().await;
        
        // Free plan
        plans.insert(
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            SaasPlan {
                plan_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                name: "Free".into(),
                slug: "free".into(),
                description: Some("Get started with sandbox mode".into()),
                price_monthly_minor: 0,
                price_per_txn_minor: 30,
                included_txns_monthly: 100,
                max_gateways: 1,
                max_team_members: 1,
                max_api_keys: 1,
                max_webhooks: 1,
                data_retention_days: 30,
                features: PlanFeatures {
                    sandbox_only: true,
                    ..PlanFeatures::default()
                },
                is_active: true,
                sort_order: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );

        // Starter plan
        plans.insert(
            Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            SaasPlan {
                plan_id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                name: "Starter".into(),
                slug: "starter".into(),
                description: Some("Perfect for small businesses".into()),
                price_monthly_minor: 9900,
                price_per_txn_minor: 15,
                included_txns_monthly: 1000,
                max_gateways: 3,
                max_team_members: 5,
                max_api_keys: 3,
                max_webhooks: 5,
                data_retention_days: 365,
                features: PlanFeatures {
                    analytics: true,
                    webhooks: true,
                    email_support: true,
                    ..PlanFeatures::default()
                },
                is_active: true,
                sort_order: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );
    }
}

// ─── SaasPlanRepository ──────────────────────────────────────────────────────

#[async_trait]
impl SaasPlanRepository for InMemorySaasBillingRepository {
    async fn find_by_id(&self, plan_id: Uuid) -> Result<Option<SaasPlan>, SaaSbillingError> {
        let plans = self.plans.read().await;
        Ok(plans.get(&plan_id).cloned())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<SaasPlan>, SaaSbillingError> {
        let plans = self.plans.read().await;
        Ok(plans.values().find(|p| p.slug == slug).cloned())
    }

    async fn list_active(&self) -> Result<Vec<SaasPlan>, SaaSbillingError> {
        let plans = self.plans.read().await;
        Ok(plans.values().filter(|p| p.is_active).cloned().collect())
    }

    async fn list_all(&self) -> Result<Vec<SaasPlan>, SaaSbillingError> {
        let plans = self.plans.read().await;
        Ok(plans.values().cloned().collect())
    }
}

// ─── TenantSubscriptionRepository ────────────────────────────────────────────

#[async_trait]
impl TenantSubscriptionRepository for InMemorySaasBillingRepository {
    async fn find_by_id(&self, subscription_id: Uuid) -> Result<Option<TenantSubscription>, SaaSbillingError> {
        let subscriptions = self.subscriptions.read().await;
        Ok(subscriptions.get(&subscription_id).cloned())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Option<TenantSubscription>, SaaSbillingError> {
        let subscriptions_by_op = self.subscriptions_by_operator.read().await;
        if let Some(sub_id) = subscriptions_by_op.get(&operator_id) {
            let subscriptions = self.subscriptions.read().await;
            Ok(subscriptions.get(sub_id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn save(&self, subscription: &mut TenantSubscription) -> Result<(), SaaSbillingError> {
        let mut subscriptions = self.subscriptions.write().await;
        let mut subscriptions_by_op = self.subscriptions_by_operator.write().await;
        
        subscriptions.insert(subscription.subscription_id, subscription.clone());
        subscriptions_by_op.insert(subscription.operator_id, subscription.subscription_id);
        Ok(())
    }

    async fn update(&self, subscription: &TenantSubscription) -> Result<(), SaaSbillingError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(subscription.subscription_id, subscription.clone());
        Ok(())
    }

    async fn count_by_plan(&self, plan_id: Uuid) -> Result<i64, SaaSbillingError> {
        let subscriptions = self.subscriptions.read().await;
        Ok(subscriptions.values().filter(|s| s.plan_id == plan_id).count() as i64)
    }
}

// ─── TenantUsageRepository ───────────────────────────────────────────────────

#[async_trait]
impl TenantUsageRepository for InMemorySaasBillingRepository {
    async fn find_by_operator_and_period(
        &self,
        operator_id: Uuid,
        period_start: Date<Utc>,
    ) -> Result<Option<TenantUsage>, SaaSbillingError> {
        let usage = self.usage.read().await;
        Ok(usage.get(&(operator_id, period_start)).cloned())
    }

    async fn save(&self, usage: &mut TenantUsage) -> Result<(), SaaSbillingError> {
        let mut usage_map = self.usage.write().await;
        usage_map.insert((usage.operator_id, usage.period_start), usage.clone());
        Ok(())
    }

    async fn update(&self, usage: &TenantUsage) -> Result<(), SaaSbillingError> {
        let mut usage_map = self.usage.write().await;
        usage_map.insert((usage.operator_id, usage.period_start), usage.clone());
        Ok(())
    }

    async fn get_current_period_usage(&self, operator_id: Uuid) -> Result<Option<TenantUsage>, SaaSbillingError> {
        let usage = self.usage.read().await;
        let today = Utc::now().date_naive();
        Ok(usage.get(&(operator_id, today)).cloned())
    }
}

// ─── SaasInvoiceRepository ───────────────────────────────────────────────────

#[async_trait]
impl SaasInvoiceRepository for InMemorySaasBillingRepository {
    async fn find_by_id(&self, invoice_id: Uuid) -> Result<Option<SaasInvoice>, SaaSbillingError> {
        let invoices = self.invoices.read().await;
        Ok(invoices.get(&invoice_id).cloned())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<SaasInvoice>, SaaSbillingError> {
        let invoices = self.invoices.read().await;
        Ok(invoices.values().filter(|i| i.operator_id == operator_id).cloned().collect())
    }

    async fn find_by_subscription(&self, subscription_id: Uuid) -> Result<Vec<SaasInvoice>, SaaSbillingError> {
        let invoices = self.invoices.read().await;
        Ok(invoices.values().filter(|i| i.subscription_id == subscription_id).cloned().collect())
    }

    async fn save(&self, invoice: &mut SaasInvoice) -> Result<(), SaaSbillingError> {
        let mut invoices = self.invoices.write().await;
        invoices.insert(invoice.invoice_id, invoice.clone());
        Ok(())
    }

    async fn update(&self, invoice: &SaasInvoice) -> Result<(), SaaSbillingError> {
        let mut invoices = self.invoices.write().await;
        invoices.insert(invoice.invoice_id, invoice.clone());
        Ok(())
    }

    async fn find_open_invoice_for_period(
        &self,
        subscription_id: Uuid,
        period_start: Date<Utc>,
    ) -> Result<Option<SaasInvoice>, SaaSbillingError> {
        let invoices = self.invoices.read().await;
        Ok(invoices.values().find(|i| {
            i.subscription_id == subscription_id
                && i.period_start == period_start
                && (i.status == InvoiceStatus::Draft || i.status == InvoiceStatus::Open)
        }).cloned())
    }
}

// ─── TeamMemberRepository ────────────────────────────────────────────────────

#[async_trait]
impl TeamMemberRepository for InMemorySaasBillingRepository {
    async fn find_by_id(&self, membership_id: Uuid) -> Result<Option<TenantTeamMember>, SaaSbillingError> {
        let members = self.team_members.read().await;
        Ok(members.get(&membership_id).cloned())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<TenantTeamMember>, SaaSbillingError> {
        let members = self.team_members.read().await;
        Ok(members.values().filter(|m| m.operator_id == operator_id).cloned().collect())
    }

    async fn find_by_principal(&self, principal_id: Uuid) -> Result<Vec<TenantTeamMember>, SaaSbillingError> {
        let members = self.team_members.read().await;
        Ok(members.values().filter(|m| m.principal_id == principal_id).cloned().collect())
    }

    async fn save(&self, member: &mut TenantTeamMember) -> Result<(), SaaSbillingError> {
        let mut members = self.team_members.write().await;
        members.insert(member.membership_id, member.clone());
        Ok(())
    }

    async fn update(&self, member: &TenantTeamMember) -> Result<(), SaaSbillingError> {
        let mut members = self.team_members.write().await;
        members.insert(member.membership_id, member.clone());
        Ok(())
    }

    async fn count_active_by_operator(&self, operator_id: Uuid) -> Result<i64, SaaSbillingError> {
        let members = self.team_members.read().await;
        Ok(members.values().filter(|m| m.operator_id == operator_id && m.status == TeamMemberStatus::Active).count() as i64)
    }
}

// ─── AuditLogRepository ──────────────────────────────────────────────────────

#[async_trait]
impl AuditLogRepository for InMemorySaasBillingRepository {
    async fn save(&self, log: &AuditLog) -> Result<(), SaaSbillingError> {
        let mut logs = self.audit_logs.write().await;
        logs.push(log.clone());
        Ok(())
    }

    async fn find_by_operator(
        &self,
        operator_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, SaaSbillingError> {
        let logs = self.audit_logs.read().await;
        Ok(logs.iter()
            .filter(|l| l.operator_id == operator_id)
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn find_by_action(
        &self,
        operator_id: Uuid,
        action: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, SaaSbillingError> {
        let logs = self.audit_logs.read().await;
        Ok(logs.iter()
            .filter(|l| l.operator_id == operator_id && l.action == action)
            .take(limit as usize)
            .cloned()
            .collect())
    }
}
