//! SaaS billing domain models for multi-tenant subscription management.
//!
//! This module defines the core types for:
//! - Subscription plans (Free, Starter, Professional, Enterprise)
//! - Tenant subscriptions (merchants subscribing to plans)
//! - Usage tracking (monthly usage per tenant)
//! - Invoices (billing invoices for merchants)

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── SaaS Plan ───────────────────────────────────────────────────────────────

/// A subscription plan that merchants can subscribe to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasPlan {
    pub plan_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub price_monthly_minor: i64,
    pub price_per_txn_minor: i64,
    pub included_txns_monthly: i32,
    pub max_gateways: i32,
    pub max_team_members: i32,
    pub max_api_keys: i32,
    pub max_webhooks: i32,
    pub data_retention_days: i32,
    pub features: PlanFeatures,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Features included in a plan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanFeatures {
    pub sandbox_only: bool,
    pub analytics: bool,
    pub webhooks: bool,
    pub email_support: bool,
    pub ai_assistant: bool,
    pub priority_support: bool,
    pub advanced_routing: bool,
    pub custom_branding: bool,
    pub sso: bool,
    pub dedicated_support: bool,
    pub sla: bool,
}

impl SaasPlan {
    /// Check if a feature is included in this plan.
    pub fn has_feature(&self, feature: &str) -> bool {
        match feature {
            "sandbox_only" => self.features.sandbox_only,
            "analytics" => self.features.analytics,
            "webhooks" => self.features.webhooks,
            "email_support" => self.features.email_support,
            "ai_assistant" => self.features.ai_assistant,
            "priority_support" => self.features.priority_support,
            "advanced_routing" => self.features.advanced_routing,
            "custom_branding" => self.features.custom_branding,
            "sso" => self.features.sso,
            "dedicated_support" => self.features.dedicated_support,
            "sla" => self.features.sla,
            _ => false,
        }
    }

    /// Check if unlimited (-1 means unlimited).
    pub fn is_unlimited(value: i32) -> bool {
        value < 0
    }
}

// ─── Tenant Subscription ─────────────────────────────────────────────────────

/// A merchant's subscription to a SaaS plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSubscription {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub plan_id: Uuid,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub payment_method_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Subscription status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Unpaid,
    Paused,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trialing => "trialing",
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Unpaid => "unpaid",
            Self::Paused => "paused",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "trialing" => Some(Self::Trialing),
            "active" => Some(Self::Active),
            "past_due" => Some(Self::PastDue),
            "canceled" => Some(Self::Canceled),
            "unpaid" => Some(Self::Unpaid),
            "paused" => Some(Self::Paused),
            _ => None,
        }
    }

    /// Check if the subscription is in a state that allows access.
    pub fn allows_access(&self) -> bool {
        matches!(self, Self::Trialing | Self::Active | Self::PastDue)
    }
}

impl TenantSubscription {
    /// Check if the subscription is currently active (including trial).
    pub fn is_active(&self) -> bool {
        self.status.allows_access()
    }

    /// Check if the subscription is in trial period.
    pub fn is_trialing(&self) -> bool {
        self.status == SubscriptionStatus::Trialing
    }

    /// Check if the trial has expired.
    pub fn is_trial_expired(&self) -> bool {
        if let Some(trial_end) = self.trial_ends_at {
            Utc::now() > trial_end
        } else {
            false
        }
    }

    /// Check if the current period has ended.
    pub fn is_period_ended(&self) -> bool {
        Utc::now() > self.current_period_end
    }
}

// ─── Tenant Usage ────────────────────────────────────────────────────────────

/// Monthly usage tracking for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    pub usage_id: Uuid,
    pub operator_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub transaction_count: i32,
    pub transaction_volume_minor: i64,
    pub api_calls: i32,
    pub storage_bytes: i64,
    pub ai_queries: i32,
    pub overage_amount_minor: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TenantUsage {
    /// Calculate overage based on plan limits.
    pub fn calculate_overage(&self, plan: &SaasPlan) -> i64 {
        let included = plan.included_txns_monthly;
        if included < 0 {
            return 0; // Unlimited
        }

        let overage_count = (self.transaction_count - included).max(0);
        overage_count as i64 * plan.price_per_txn_minor
    }
}

// ─── SaaS Invoice ────────────────────────────────────────────────────────────

/// An invoice for a merchant's SaaS subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasInvoice {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub subscription_id: Uuid,
    pub invoice_number: String,
    pub status: InvoiceStatus,
    pub subtotal_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub currency: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub due_date: NaiveDate,
    pub paid_at: Option<DateTime<Utc>>,
    pub line_items: Vec<InvoiceLineItem>,
    pub stripe_invoice_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Invoice status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

impl InvoiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::Paid => "paid",
            Self::Void => "void",
            Self::Uncollectible => "uncollectible",
        }
    }
}

/// A line item on an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub description: String,
    pub amount_minor: i64,
    pub quantity: i32,
    pub unit_price_minor: i64,
}

// ─── Team Member ─────────────────────────────────────────────────────────────

/// A team member invitation for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantTeamMember {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub invited_by: Uuid,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub status: TeamMemberStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Team member status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamMemberStatus {
    Pending,
    Active,
    Suspended,
}

impl TeamMemberStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Suspended => "suspended",
        }
    }
}

// ─── Audit Log ───────────────────────────────────────────────────────────────

/// An audit log entry for compliance tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub log_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// ─── DTOs ────────────────────────────────────────────────────────────────────

/// Request to create a new tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantSubscriptionRequest {
    pub operator_id: Uuid,
    pub plan_id: Uuid,
    pub payment_method_id: Option<String>,
    pub trial_days: Option<i32>,
}

/// Request to update a tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTenantSubscriptionRequest {
    pub plan_id: Option<Uuid>,
    pub payment_method_id: Option<String>,
}

/// Request to cancel a tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTenantSubscriptionRequest {
    pub reason: Option<String>,
    pub cancel_at_period_end: bool,
}

/// Response for tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSubscriptionResponse {
    pub subscription: TenantSubscription,
    pub plan: SaasPlan,
    pub usage: Option<TenantUsage>,
}

/// Request to get usage for a specific period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUsageRequest {
    pub operator_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
}

/// Response for usage summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummaryResponse {
    pub usage: TenantUsage,
    pub plan: SaasPlan,
    pub overage_amount: i64,
    pub total_amount: i64,
}

/// Request to create an audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLogRequest {
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
