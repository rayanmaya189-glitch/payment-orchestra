//! Tenant Usage domain model.

use chrono::{Date, DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::plan::SaasPlan;

/// Monthly usage tracking for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    pub usage_id: Uuid,
    pub operator_id: Uuid,
    pub period_start: Date<Utc>,
    pub period_end: Date<Utc>,
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
    /// Create a new usage record for a period.
    pub fn new(operator_id: Uuid, period_start: Date<Utc>, period_end: Date<Utc>) -> Self {
        let now = Utc::now();
        Self {
            usage_id: Uuid::now_v7(),
            operator_id,
            period_start,
            period_end,
            transaction_count: 0,
            transaction_volume_minor: 0,
            api_calls: 0,
            storage_bytes: 0,
            ai_queries: 0,
            overage_amount_minor: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Increment transaction count.
    pub fn record_transaction(&mut self, amount_minor: i64) {
        self.transaction_count += 1;
        self.transaction_volume_minor += amount_minor;
        self.updated_at = Utc::now();
    }

    /// Increment API call count.
    pub fn record_api_call(&mut self) {
        self.api_calls += 1;
        self.updated_at = Utc::now();
    }

    /// Increment storage usage.
    pub fn record_storage(&mut self, bytes: i64) {
        self.storage_bytes += bytes;
        self.updated_at = Utc::now();
    }

    /// Increment AI query count.
    pub fn record_ai_query(&mut self) {
        self.ai_queries += 1;
        self.updated_at = Utc::now();
    }

    /// Calculate overage based on plan limits.
    pub fn calculate_overage(&self, plan: &SaasPlan) -> i64 {
        let included = plan.included_txns_monthly;
        if included < 0 {
            return 0; // Unlimited
        }

        let overage_count = (self.transaction_count - included).max(0);
        overage_count as i64 * plan.price_per_txn_minor
    }

    /// Calculate total amount (base price + overage).
    pub fn calculate_total(&self, plan: &SaasPlan) -> i64 {
        let base = plan.price_monthly_minor;
        let overage = self.calculate_overage(plan);
        base + overage
    }

    /// Check if usage exceeds plan limits.
    pub fn exceeds_limit(&self, plan: &SaasPlan) -> bool {
        if SaasPlan::is_unlimited(plan.included_txns_monthly) {
            return false;
        }
        self.transaction_count > plan.included_txns_monthly
    }
}

impl std::fmt::Display for TenantUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Usage({} txns, {} API calls, {} storage)",
            self.transaction_count, self.api_calls, self.storage_bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{PlanFeatures, SaasPlan};

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
            features: PlanFeatures::default(),
            is_active: true,
            sort_order: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn test_usage() -> TenantUsage {
        TenantUsage::new(
            Uuid::now_v7(),
            chrono::Utc::now().date_naive(),
            chrono::Utc::now().date_naive() + chrono::Duration::days(30),
        )
    }

    #[test]
    fn test_usage_record_transaction() {
        let mut usage = test_usage();
        usage.record_transaction(1000);
        assert_eq!(usage.transaction_count, 1);
        assert_eq!(usage.transaction_volume_minor, 1000);

        usage.record_transaction(2000);
        assert_eq!(usage.transaction_count, 2);
        assert_eq!(usage.transaction_volume_minor, 3000);
    }

    #[test]
    fn test_usage_record_api_call() {
        let mut usage = test_usage();
        usage.record_api_call();
        usage.record_api_call();
        assert_eq!(usage.api_calls, 2);
    }

    #[test]
    fn test_usage_calculate_overage() {
        let plan = test_plan();
        let mut usage = test_usage();
        usage.transaction_count = 150;

        // 150 transactions - 100 included = 50 overage
        // 50 * 30 minor units = 1500
        assert_eq!(usage.calculate_overage(&plan), 1500);
    }

    #[test]
    fn test_usage_calculate_overage_unlimited() {
        let mut plan = test_plan();
        plan.included_txns_monthly = -1;
        let mut usage = test_usage();
        usage.transaction_count = 1000;

        assert_eq!(usage.calculate_overage(&plan), 0);
    }

    #[test]
    fn test_usage_calculate_total() {
        let plan = test_plan();
        let mut usage = test_usage();
        usage.transaction_count = 150;

        // Base: 0 + Overage: 1500 = 1500
        assert_eq!(usage.calculate_total(&plan), 1500);
    }

    #[test]
    fn test_usage_exceeds_limit() {
        let plan = test_plan();
        let mut usage = test_usage();
        usage.transaction_count = 100;
        assert!(!usage.exceeds_limit(&plan));

        usage.transaction_count = 101;
        assert!(usage.exceeds_limit(&plan));
    }

    #[test]
    fn test_usage_display() {
        let mut usage = test_usage();
        usage.transaction_count = 50;
        usage.api_calls = 100;
        usage.storage_bytes = 1024;
        assert_eq!(format!("{}", usage), "Usage(50 txns, 100 API calls, 1024 storage)");
    }
}
