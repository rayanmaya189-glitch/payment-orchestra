//! SaaS Billing domain model — Multi-tenant subscription management.
//!
//! File structure (one concept per file per CONVENTIONS.md):
//!
//! - [`error`]                — [`SaaSbillingError`]
//! - [`plan`]                 — [`SaasPlan`], [`PlanFeatures`]
//! - [`subscription`]         — [`TenantSubscription`], [`SubscriptionStatus`]
//! - [`usage`]                — [`TenantUsage`]
//! - [`invoice`]              — [`SaasInvoice`], [`InvoiceStatus`]
//! - [`team`]                 — [`TenantTeamMember`], [`TeamMemberStatus`]
//! - [`audit`]                — [`AuditLog`]

pub mod error;
pub mod plan;
pub mod subscription;
pub mod usage;
pub mod invoice;
pub mod team;
pub mod audit;
pub mod stripe_client;

pub use error::*;
pub use plan::*;
pub use subscription::*;
pub use usage::*;
pub use invoice::*;
pub use team::*;
pub use audit::*;
pub use stripe_client::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_plan() -> SaasPlan {
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
                analytics: false,
                webhooks: false,
                email_support: false,
                ai_assistant: false,
                priority_support: false,
                advanced_routing: false,
                custom_branding: false,
                sso: false,
                dedicated_support: false,
                sla: false,
            },
            is_active: true,
            sort_order: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn test_subscription() -> TenantSubscription {
        TenantSubscription {
            subscription_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            plan_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            status: SubscriptionStatus::Active,
            current_period_start: chrono::Utc::now(),
            current_period_end: chrono::Utc::now() + chrono::Duration::days(30),
            trial_ends_at: None,
            canceled_at: None,
            cancel_reason: None,
            payment_method_id: None,
            stripe_subscription_id: None,
            created_by: Uuid::now_v7(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_plan_has_feature() {
        let plan = test_plan();
        assert!(plan.has_feature("sandbox_only"));
        assert!(!plan.has_feature("analytics"));
        assert!(!plan.has_feature("nonexistent"));
    }

    #[test]
    fn test_plan_is_unlimited() {
        assert!(SaasPlan::is_unlimited(-1));
        assert!(!SaasPlan::is_unlimited(100));
        assert!(!SaasPlan::is_unlimited(0));
    }

    #[test]
    fn test_subscription_is_active() {
        let mut sub = test_subscription();
        assert!(sub.is_active());

        sub.status = SubscriptionStatus::Canceled;
        assert!(!sub.is_active());

        sub.status = SubscriptionStatus::Trialing;
        assert!(sub.is_active());
    }

    #[test]
    fn test_subscription_is_trialing() {
        let mut sub = test_subscription();
        assert!(!sub.is_trialing());

        sub.status = SubscriptionStatus::Trialing;
        assert!(sub.is_trialing());
    }

    #[test]
    fn test_subscription_is_trial_expired() {
        let mut sub = test_subscription();
        sub.status = SubscriptionStatus::Trialing;
        sub.trial_ends_at = Some(chrono::Utc::now() - chrono::Duration::days(1));
        assert!(sub.is_trial_expired());

        sub.trial_ends_at = Some(chrono::Utc::now() + chrono::Duration::days(7));
        assert!(!sub.is_trial_expired());
    }

    #[test]
    fn test_subscription_status_allows_access() {
        assert!(SubscriptionStatus::Trialing.allows_access());
        assert!(SubscriptionStatus::Active.allows_access());
        assert!(SubscriptionStatus::PastDue.allows_access());
        assert!(!SubscriptionStatus::Canceled.allows_access());
        assert!(!SubscriptionStatus::Unpaid.allows_access());
        assert!(!SubscriptionStatus::Paused.allows_access());
    }

    #[test]
    fn test_usage_calculate_overage() {
        let plan = test_plan();
        let usage = TenantUsage {
            usage_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            period_start: chrono::Utc::now().date_naive(),
            period_end: chrono::Utc::now().date_naive() + chrono::Duration::days(30),
            transaction_count: 150,
            transaction_volume_minor: 15000,
            api_calls: 500,
            storage_bytes: 1024,
            ai_queries: 10,
            overage_amount_minor: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // 150 transactions - 100 included = 50 overage
        // 50 * 30 minor units = 1500
        assert_eq!(usage.calculate_overage(&plan), 1500);
    }
}
