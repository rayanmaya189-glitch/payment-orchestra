//! SaaS Plan domain model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

    /// Check if the plan allows production access (not sandbox-only).
    pub fn allows_production(&self) -> bool {
        !self.features.sandbox_only
    }

    /// Check if the plan includes a specific limit.
    pub fn check_limit(&self, current: i32, limit: i32) -> bool {
        if SaasPlan::is_unlimited(limit) {
            return true;
        }
        current < limit
    }
}

impl std::fmt::Display for SaasPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_plan_allows_production() {
        let plan = test_plan();
        assert!(!plan.allows_production());

        let mut prod_plan = test_plan();
        prod_plan.features.sandbox_only = false;
        assert!(prod_plan.allows_production());
    }

    #[test]
    fn test_plan_check_limit() {
        let plan = test_plan();
        assert!(plan.check_limit(0, 1));
        assert!(!plan.check_limit(1, 1));
        assert!(plan.check_limit(100, -1)); // Unlimited
    }

    #[test]
    fn test_plan_display() {
        let plan = test_plan();
        assert_eq!(format!("{}", plan), "Free (free)");
    }
}
