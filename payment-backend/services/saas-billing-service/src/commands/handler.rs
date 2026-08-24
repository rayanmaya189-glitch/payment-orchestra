//! Command handler implementation for SaaS Billing service.

use async_trait::async_trait;
use uuid::Uuid;
use chrono::{Datelike, Timelike, Utc};

use crate::domain::*;
use crate::repository::traits::*;
use super::types::*;

/// Command handler trait for SaaS Billing.
#[async_trait]
pub trait CommandHandler: Send + Sync {
    // Subscription commands
    async fn create_subscription(&self, cmd: CreateTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError>;
    async fn update_subscription(&self, cmd: UpdateTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError>;
    async fn cancel_subscription(&self, cmd: CancelTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError>;
    async fn activate_subscription(&self, cmd: ActivateTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError>;
    async fn pause_subscription(&self, cmd: PauseTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError>;
    async fn resume_subscription(&self, cmd: ResumeTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError>;

    // Usage commands
    async fn record_transaction_usage(&self, cmd: RecordTransactionUsageCommand) -> Result<TenantUsage, SaaSbillingError>;
    async fn record_api_call_usage(&self, cmd: RecordApiCallUsageCommand) -> Result<TenantUsage, SaaSbillingError>;
    async fn record_storage_usage(&self, cmd: RecordStorageUsageCommand) -> Result<TenantUsage, SaaSbillingError>;
    async fn record_ai_query_usage(&self, cmd: RecordAiQueryUsageCommand) -> Result<TenantUsage, SaaSbillingError>;

    // Invoice commands
    async fn create_invoice(&self, cmd: CreateInvoiceCommand) -> Result<SaasInvoice, SaaSbillingError>;
    async fn finalize_invoice(&self, cmd: FinalizeInvoiceCommand) -> Result<SaasInvoice, SaaSbillingError>;
    async fn mark_invoice_paid(&self, cmd: MarkInvoicePaidCommand) -> Result<SaasInvoice, SaaSbillingError>;

    // Team commands
    async fn invite_team_member(&self, cmd: InviteTeamMemberCommand) -> Result<TenantTeamMember, SaaSbillingError>;
    async fn accept_team_member(&self, cmd: AcceptTeamMemberCommand) -> Result<TenantTeamMember, SaaSbillingError>;
    async fn suspend_team_member(&self, cmd: SuspendTeamMemberCommand) -> Result<TenantTeamMember, SaaSbillingError>;
    async fn change_team_member_role(&self, cmd: ChangeTeamMemberRoleCommand) -> Result<TenantTeamMember, SaaSbillingError>;

    // Audit commands
    async fn create_audit_log(&self, cmd: CreateAuditLogCommand) -> Result<AuditLog, SaaSbillingError>;
}

/// SaaS Billing command handler implementation.
pub struct SaasBillingCommandHandler<P, S, U, I, T, A> {
    plan_repo: P,
    subscription_repo: S,
    usage_repo: U,
    invoice_repo: I,
    team_repo: T,
    audit_repo: A,
}

impl<P, S, U, I, T, A> SaasBillingCommandHandler<P, S, U, I, T, A> {
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
impl<P, S, U, I, T, A> CommandHandler for SaasBillingCommandHandler<P, S, U, I, T, A>
where
    P: SaasPlanRepository,
    S: TenantSubscriptionRepository,
    U: TenantUsageRepository,
    I: SaasInvoiceRepository,
    T: TeamMemberRepository,
    A: AuditLogRepository,
{
    // ─── Subscription Commands ───────────────────────────────────────────────

    async fn create_subscription(&self, cmd: CreateTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError> {
        // Check if subscription already exists
        if let Some(_) = self.subscription_repo.find_by_operator(cmd.operator_id).await? {
            return Err(SaaSbillingError::SubscriptionAlreadyExists(cmd.operator_id));
        }

        // Verify plan exists and is active
        let plan = self.plan_repo.find_by_id(cmd.plan_id).await?
            .ok_or_else(|| SaaSbillingError::PlanNotFound(cmd.plan_id.to_string()))?;
        
        if !plan.is_active {
            return Err(SaaSbillingError::PlanInactive(plan.slug));
        }

        // Create subscription
        let mut subscription = TenantSubscription::new(
            cmd.operator_id,
            cmd.plan_id,
            cmd.created_by,
            cmd.trial_days,
        );

        if let Some(pm_id) = cmd.payment_method_id {
            subscription.payment_method_id = Some(pm_id);
        }

        self.subscription_repo.save(&mut subscription).await?;

        // Create audit log
        let _ = self.create_audit_log(CreateAuditLogCommand {
            operator_id: cmd.operator_id,
            principal_id: cmd.created_by,
            action: AuditActions::SUBSCRIPTION_CREATED.into(),
            resource: "subscription".into(),
            resource_id: Some(subscription.subscription_id.to_string()),
            old_value: None,
            new_value: Some(serde_json::to_value(&subscription).unwrap_or_default()),
            ip_address: None,
            user_agent: None,
            metadata: None,
        }).await;

        Ok(subscription)
    }

    async fn update_subscription(&self, cmd: UpdateTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError> {
        let mut subscription = self.subscription_repo.find_by_id(cmd.subscription_id).await?
            .ok_or_else(|| SaaSbillingError::SubscriptionNotFound(cmd.subscription_id))?;

        // Verify ownership
        if subscription.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        // Update plan if provided
        if let Some(plan_id) = cmd.plan_id {
            let plan = self.plan_repo.find_by_id(plan_id).await?
                .ok_or_else(|| SaaSbillingError::PlanNotFound(plan_id.to_string()))?;
            
            if !plan.is_active {
                return Err(SaaSbillingError::PlanInactive(plan.slug));
            }

            subscription.plan_id = plan_id;
        }

        // Update payment method if provided
        if let Some(pm_id) = cmd.payment_method_id {
            subscription.payment_method_id = Some(pm_id);
        }

        subscription.updated_at = Utc::now();
        self.subscription_repo.update(&subscription).await?;

        Ok(subscription)
    }

    async fn cancel_subscription(&self, cmd: CancelTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError> {
        let mut subscription = self.subscription_repo.find_by_id(cmd.subscription_id).await?
            .ok_or_else(|| SaaSbillingError::SubscriptionNotFound(cmd.subscription_id))?;

        // Verify ownership
        if subscription.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        subscription.cancel(cmd.reason)?;
        self.subscription_repo.update(&subscription).await?;

        // Create audit log
        let _ = self.create_audit_log(CreateAuditLogCommand {
            operator_id: cmd.operator_id,
            principal_id: cmd.operator_id, // TODO: Get from context
            action: AuditActions::SUBSCRIPTION_CANCELED.into(),
            resource: "subscription".into(),
            resource_id: Some(subscription.subscription_id.to_string()),
            old_value: None,
            new_value: Some(serde_json::to_value(&subscription).unwrap_or_default()),
            ip_address: None,
            user_agent: None,
            metadata: None,
        }).await;

        Ok(subscription)
    }

    async fn activate_subscription(&self, cmd: ActivateTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError> {
        let mut subscription = self.subscription_repo.find_by_id(cmd.subscription_id).await?
            .ok_or_else(|| SaaSbillingError::SubscriptionNotFound(cmd.subscription_id))?;

        // Verify ownership
        if subscription.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        subscription.activate()?;
        self.subscription_repo.update(&subscription).await?;

        Ok(subscription)
    }

    async fn pause_subscription(&self, cmd: PauseTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError> {
        let mut subscription = self.subscription_repo.find_by_id(cmd.subscription_id).await?
            .ok_or_else(|| SaaSbillingError::SubscriptionNotFound(cmd.subscription_id))?;

        // Verify ownership
        if subscription.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        subscription.pause()?;
        self.subscription_repo.update(&subscription).await?;

        Ok(subscription)
    }

    async fn resume_subscription(&self, cmd: ResumeTenantSubscriptionCommand) -> Result<TenantSubscription, SaaSbillingError> {
        let mut subscription = self.subscription_repo.find_by_id(cmd.subscription_id).await?
            .ok_or_else(|| SaaSbillingError::SubscriptionNotFound(cmd.subscription_id))?;

        // Verify ownership
        if subscription.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        subscription.resume()?;
        self.subscription_repo.update(&subscription).await?;

        Ok(subscription)
    }

    // ─── Usage Commands ──────────────────────────────────────────────────────

    async fn record_transaction_usage(&self, cmd: RecordTransactionUsageCommand) -> Result<TenantUsage, SaaSbillingError> {
        let now = Utc::now();
        let year = now.year();
        let month = now.month();
        let period_start = Utc::now().with_day(1).and_then(|d| d.with_hour(0)).and_then(|d| d.with_minute(0)).and_then(|d| d.with_second(0)).unwrap_or(now);
        
        let mut usage = if let Some(existing) = self.usage_repo.find_by_operator_and_period(cmd.operator_id, period_start).await? {
            existing
        } else {
            let next_month = if month == 12 { 1 } else { month + 1 };
            let next_year = if month == 12 { year + 1 } else { year };
            let period_end = Utc::now().with_year(next_year).and_then(|d| d.with_month(next_month)).and_then(|d| d.with_day(1)).unwrap_or(now);
            TenantUsage::new(cmd.operator_id, period_start, period_end)
        };

        usage.record_transaction(cmd.amount_minor);
        self.usage_repo.update(&usage).await?;

        Ok(usage)
    }

    async fn record_api_call_usage(&self, cmd: RecordApiCallUsageCommand) -> Result<TenantUsage, SaaSbillingError> {
        let now = Utc::now();
        let year = now.year();
        let month = now.month();
        let period_start = Utc::now().with_day(1).and_then(|d| d.with_hour(0)).and_then(|d| d.with_minute(0)).and_then(|d| d.with_second(0)).unwrap_or(now);
        
        let mut usage = if let Some(existing) = self.usage_repo.find_by_operator_and_period(cmd.operator_id, period_start).await? {
            existing
        } else {
            let next_month = if month == 12 { 1 } else { month + 1 };
            let next_year = if month == 12 { year + 1 } else { year };
            let period_end = Utc::now().with_year(next_year).and_then(|d| d.with_month(next_month)).and_then(|d| d.with_day(1)).unwrap_or(now);
            TenantUsage::new(cmd.operator_id, period_start, period_end)
        };

        usage.record_api_call();
        self.usage_repo.update(&usage).await?;

        Ok(usage)
    }

    async fn record_storage_usage(&self, cmd: RecordStorageUsageCommand) -> Result<TenantUsage, SaaSbillingError> {
        let now = Utc::now();
        let year = now.year();
        let month = now.month();
        let period_start = Utc::now().with_day(1).and_then(|d| d.with_hour(0)).and_then(|d| d.with_minute(0)).and_then(|d| d.with_second(0)).unwrap_or(now);
        
        let mut usage = if let Some(existing) = self.usage_repo.find_by_operator_and_period(cmd.operator_id, period_start).await? {
            existing
        } else {
            let next_month = if month == 12 { 1 } else { month + 1 };
            let next_year = if month == 12 { year + 1 } else { year };
            let period_end = Utc::now().with_year(next_year).and_then(|d| d.with_month(next_month)).and_then(|d| d.with_day(1)).unwrap_or(now);
            TenantUsage::new(cmd.operator_id, period_start, period_end)
        };

        usage.record_storage(cmd.bytes);
        self.usage_repo.update(&usage).await?;

        Ok(usage)
    }

    async fn record_ai_query_usage(&self, cmd: RecordAiQueryUsageCommand) -> Result<TenantUsage, SaaSbillingError> {
        let now = Utc::now();
        let year = now.year();
        let month = now.month();
        let period_start = Utc::now().with_day(1).and_then(|d| d.with_hour(0)).and_then(|d| d.with_minute(0)).and_then(|d| d.with_second(0)).unwrap_or(now);
        
        let mut usage = if let Some(existing) = self.usage_repo.find_by_operator_and_period(cmd.operator_id, period_start).await? {
            existing
        } else {
            let next_month = if month == 12 { 1 } else { month + 1 };
            let next_year = if month == 12 { year + 1 } else { year };
            let period_end = Utc::now().with_year(next_year).and_then(|d| d.with_month(next_month)).and_then(|d| d.with_day(1)).unwrap_or(now);
            TenantUsage::new(cmd.operator_id, period_start, period_end)
        };

        usage.record_ai_query();
        self.usage_repo.update(&usage).await?;

        Ok(usage)
    }

    // ─── Invoice Commands ────────────────────────────────────────────────────

    async fn create_invoice(&self, cmd: CreateInvoiceCommand) -> Result<SaasInvoice, SaaSbillingError> {
        // Check if invoice already exists for this period
        if let Some(_) = self.invoice_repo.find_open_invoice_for_period(cmd.subscription_id, cmd.period_start).await? {
            return Err(SaaSbillingError::ValidationError("Invoice already exists for this period".into()));
        }

        // Get subscription and plan
        let subscription = self.subscription_repo.find_by_id(cmd.subscription_id).await?
            .ok_or_else(|| SaaSbillingError::SubscriptionNotFound(cmd.subscription_id))?;
        
        let plan = self.plan_repo.find_by_id(subscription.plan_id).await?
            .ok_or_else(|| SaaSbillingError::PlanNotFound(subscription.plan_id.to_string()))?;

        // Get usage for the period
        let usage = self.usage_repo.find_by_operator_and_period(cmd.operator_id, cmd.period_start).await?;

        // Build line items
        let mut line_items = Vec::new();
        
        // Base subscription fee
        if plan.price_monthly_minor > 0 {
            line_items.push(InvoiceLineItem {
                description: format!("{} - Monthly Subscription", plan.name),
                amount_minor: plan.price_monthly_minor,
                quantity: 1,
                unit_price_minor: plan.price_monthly_minor,
            });
        }

        // Usage overage
        if let Some(ref usage) = usage {
            let overage = usage.calculate_overage(&plan);
            if overage > 0 {
                let included = plan.included_txns_monthly.max(0) as i64;
                let overage_count = (usage.transaction_count as i64 - included).max(0);
                line_items.push(InvoiceLineItem {
                    description: format!("Transaction Overage ({} transactions)", overage_count),
                    amount_minor: overage,
                    quantity: overage_count as i32,
                    unit_price_minor: plan.price_per_txn_minor,
                });
            }
        }

        // Generate invoice number
        let invoice_count = self.invoice_repo.find_by_operator(cmd.operator_id).await?.len();
        let invoice_number = format!("INV-{}-{:04}", Utc::now().format("%Y"), invoice_count + 1);

        let mut invoice = SaasInvoice::new(
            cmd.operator_id,
            cmd.subscription_id,
            invoice_number,
            cmd.period_start,
            cmd.period_end,
            line_items,
        );

        self.invoice_repo.save(&mut invoice).await?;

        Ok(invoice)
    }

    async fn finalize_invoice(&self, cmd: FinalizeInvoiceCommand) -> Result<SaasInvoice, SaaSbillingError> {
        let mut invoice = self.invoice_repo.find_by_id(cmd.invoice_id).await?
            .ok_or_else(|| SaaSbillingError::InvoiceNotFound(cmd.invoice_id))?;

        // Verify ownership
        if invoice.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        invoice.finalize()?;
        self.invoice_repo.update(&invoice).await?;

        Ok(invoice)
    }

    async fn mark_invoice_paid(&self, cmd: MarkInvoicePaidCommand) -> Result<SaasInvoice, SaaSbillingError> {
        let mut invoice = self.invoice_repo.find_by_id(cmd.invoice_id).await?
            .ok_or_else(|| SaaSbillingError::InvoiceNotFound(cmd.invoice_id))?;

        // Verify ownership
        if invoice.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        if let Some(stripe_id) = cmd.stripe_invoice_id {
            invoice.stripe_invoice_id = Some(stripe_id);
        }

        invoice.mark_paid()?;
        self.invoice_repo.update(&invoice).await?;

        Ok(invoice)
    }

    // ─── Team Commands ───────────────────────────────────────────────────────

    async fn invite_team_member(&self, cmd: InviteTeamMemberCommand) -> Result<TenantTeamMember, SaaSbillingError> {
        // Check team member limit
        let subscription = self.subscription_repo.find_by_operator(cmd.operator_id).await?;
        let plan = if let Some(ref sub) = subscription {
            self.plan_repo.find_by_id(sub.plan_id).await?
        } else {
            None
        };

        if let Some(ref plan) = plan {
            let current_count = self.team_repo.count_active_by_operator(cmd.operator_id).await?;
            if !plan.check_limit(current_count as i32, plan.max_team_members) {
                return Err(SaaSbillingError::TeamMemberLimitExceeded {
                    limit: plan.max_team_members,
                });
            }
        }

        // Check if user is already a member
        let existing_members = self.team_repo.find_by_operator(cmd.operator_id).await?;
        // TODO: Check by email when we have user lookup

        let mut member = TenantTeamMember::new(
            cmd.operator_id,
            Uuid::nil(), // Will be set when invitation is accepted
            &cmd.role,
            cmd.invited_by,
        );

        self.team_repo.save(&mut member).await?;

        // Create audit log
        let _ = self.create_audit_log(CreateAuditLogCommand {
            operator_id: cmd.operator_id,
            principal_id: cmd.invited_by,
            action: AuditActions::TEAM_MEMBER_INVITED.into(),
            resource: "team_member".into(),
            resource_id: Some(member.membership_id.to_string()),
            old_value: None,
            new_value: Some(serde_json::to_value(&member).unwrap_or_default()),
            ip_address: None,
            user_agent: None,
            metadata: Some(serde_json::json!({"email": cmd.email, "role": cmd.role})),
        }).await;

        Ok(member)
    }

    async fn accept_team_member(&self, cmd: AcceptTeamMemberCommand) -> Result<TenantTeamMember, SaaSbillingError> {
        let mut member = self.team_repo.find_by_id(cmd.membership_id).await?
            .ok_or_else(|| SaaSbillingError::TeamMemberNotFound(cmd.membership_id))?;

        member.principal_id = cmd.principal_id;
        member.accept()?;
        self.team_repo.update(&member).await?;

        // Create audit log
        let _ = self.create_audit_log(CreateAuditLogCommand {
            operator_id: member.operator_id,
            principal_id: cmd.principal_id,
            action: AuditActions::TEAM_MEMBER_ACCEPTED.into(),
            resource: "team_member".into(),
            resource_id: Some(member.membership_id.to_string()),
            old_value: None,
            new_value: Some(serde_json::to_value(&member).unwrap_or_default()),
            ip_address: None,
            user_agent: None,
            metadata: None,
        }).await;

        Ok(member)
    }

    async fn suspend_team_member(&self, cmd: SuspendTeamMemberCommand) -> Result<TenantTeamMember, SaaSbillingError> {
        let mut member = self.team_repo.find_by_id(cmd.membership_id).await?
            .ok_or_else(|| SaaSbillingError::TeamMemberNotFound(cmd.membership_id))?;

        // Verify ownership
        if member.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        member.suspend()?;
        self.team_repo.update(&member).await?;

        // Create audit log
        let _ = self.create_audit_log(CreateAuditLogCommand {
            operator_id: cmd.operator_id,
            principal_id: cmd.operator_id, // TODO: Get from context
            action: AuditActions::TEAM_MEMBER_SUSPENDED.into(),
            resource: "team_member".into(),
            resource_id: Some(member.membership_id.to_string()),
            old_value: None,
            new_value: Some(serde_json::to_value(&member).unwrap_or_default()),
            ip_address: None,
            user_agent: None,
            metadata: None,
        }).await;

        Ok(member)
    }

    async fn change_team_member_role(&self, cmd: ChangeTeamMemberRoleCommand) -> Result<TenantTeamMember, SaaSbillingError> {
        let mut member = self.team_repo.find_by_id(cmd.membership_id).await?
            .ok_or_else(|| SaaSbillingError::TeamMemberNotFound(cmd.membership_id))?;

        // Verify ownership
        if member.operator_id != cmd.operator_id {
            return Err(SaaSbillingError::ValidationError("Unauthorized".into()));
        }

        let old_role = member.role.clone();
        member.change_role(&cmd.new_role)?;
        self.team_repo.update(&member).await?;

        // Create audit log
        let _ = self.create_audit_log(CreateAuditLogCommand {
            operator_id: cmd.operator_id,
            principal_id: cmd.operator_id, // TODO: Get from context
            action: AuditActions::TEAM_MEMBER_ROLE_CHANGED.into(),
            resource: "team_member".into(),
            resource_id: Some(member.membership_id.to_string()),
            old_value: Some(serde_json::json!({"role": old_role})),
            new_value: Some(serde_json::json!({"role": cmd.new_role})),
            ip_address: None,
            user_agent: None,
            metadata: None,
        }).await;

        Ok(member)
    }

    // ─── Audit Commands ──────────────────────────────────────────────────────

    async fn create_audit_log(&self, cmd: CreateAuditLogCommand) -> Result<AuditLog, SaaSbillingError> {
        let mut log = AuditLog::new(
            cmd.operator_id,
            cmd.principal_id,
            &cmd.action,
            &cmd.resource,
            cmd.resource_id,
        );

        log.old_value = cmd.old_value;
        log.new_value = cmd.new_value;
        log.ip_address = cmd.ip_address;
        log.user_agent = cmd.user_agent;
        log.metadata = cmd.metadata;

        self.audit_repo.save(&log).await?;

        Ok(log)
    }
}
