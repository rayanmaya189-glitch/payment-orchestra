//! Usage-based billing metering — track API calls, transactions, and features.
//!
//! Provides:
//! - Real-time usage tracking per tenant
//! - Usage-based billing calculations
//! - Overage detection and alerts
//! - Usage analytics and reporting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ─── Usage Types ─────────────────────────────────────────────────────────────

/// Types of billable usage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UsageType {
    ApiCalls,
    Transactions,
    ConnectorCalls,
    StorageBytes,
    WebhookDeliveries,
    AiQueries,
    TeamMembers,
    Custom(String),
}

impl UsageType {
    pub fn as_str(&self) -> &str {
        match self {
            UsageType::ApiCalls => "api_calls",
            UsageType::Transactions => "transactions",
            UsageType::ConnectorCalls => "connector_calls",
            UsageType::StorageBytes => "storage_bytes",
            UsageType::WebhookDeliveries => "webhook_deliveries",
            UsageType::AiQueries => "ai_queries",
            UsageType::TeamMembers => "team_members",
            UsageType::Custom(name) => name,
        }
    }
}

/// Usage unit for billing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsageUnit {
    Count,
    Bytes,
    Seconds,
    Requests,
}

/// A usage record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub record_id: Uuid,
    pub tenant_id: Uuid,
    pub usage_type: UsageType,
    pub quantity: u64,
    pub unit: UsageUnit,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// Aggregated usage for a tenant in a billing period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    pub tenant_id: Uuid,
    pub billing_period_start: DateTime<Utc>,
    pub billing_period_end: DateTime<Utc>,
    pub usage_by_type: HashMap<String, UsageAggregation>,
    pub total_estimated_cost: i64, // In minor units
}

/// Aggregation for a specific usage type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAggregation {
    pub usage_type: String,
    pub total_quantity: u64,
    pub included_quantity: u64, // Free tier included
    pub billable_quantity: u64, // quantity - included
    pub unit_price: i64,        // Price per unit in minor units
    pub total_cost: i64,        // billable_quantity * unit_price
}

// ─── Pricing Configuration ───────────────────────────────────────────────────

/// Pricing tier configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTier {
    pub tier_name: String,
    pub monthly_price: i64, // Base monthly price in minor units
    pub usage_limits: HashMap<String, UsageLimit>,
    pub overage_rates: HashMap<String, i64>, // Price per unit for overages
}

/// Usage limit for a specific type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLimit {
    pub included_quantity: u64,
    pub unit: UsageUnit,
    pub hard_limit: Option<u64>, // If exceeded, service is restricted
}

// ─── Metering Service ────────────────────────────────────────────────────────

/// Service for tracking and billing usage.
pub struct UsageMeteringService {
    /// Pricing tiers
    pricing_tiers: HashMap<String, PricingTier>,
    /// Usage records buffer (in production, this would be a database)
    records: Vec<UsageRecord>,
}

impl UsageMeteringService {
    /// Create a new metering service with default pricing.
    pub fn new() -> Self {
        let mut pricing_tiers = HashMap::new();

        // Starter plan
        pricing_tiers.insert(
            "starter".into(),
            PricingTier {
                tier_name: "Starter".into(),
                monthly_price: 9900, // $99.00
                usage_limits: HashMap::from([
                    (
                        "api_calls".into(),
                        UsageLimit {
                            included_quantity: 10000,
                            unit: UsageUnit::Count,
                            hard_limit: Some(50000),
                        },
                    ),
                    (
                        "transactions".into(),
                        UsageLimit {
                            included_quantity: 10000,
                            unit: UsageUnit::Count,
                            hard_limit: None,
                        },
                    ),
                ]),
                overage_rates: HashMap::from([
                    ("api_calls".into(), 10), // $0.001 per call
                    ("transactions".into(), 50), // $0.005 per transaction
                ]),
            },
        );

        // Growth plan
        pricing_tiers.insert(
            "growth".into(),
            PricingTier {
                tier_name: "Growth".into(),
                monthly_price: 49900, // $499.00
                usage_limits: HashMap::from([
                    (
                        "api_calls".into(),
                        UsageLimit {
                            included_quantity: 100000,
                            unit: UsageUnit::Count,
                            hard_limit: Some(500000),
                        },
                    ),
                    (
                        "transactions".into(),
                        UsageLimit {
                            included_quantity: 100000,
                            unit: UsageUnit::Count,
                            hard_limit: None,
                        },
                    ),
                ]),
                overage_rates: HashMap::from([
                    ("api_calls".into(), 5), // $0.0005 per call
                    ("transactions".into(), 30), // $0.003 per transaction
                ]),
            },
        );

        // Enterprise plan
        pricing_tiers.insert(
            "enterprise".into(),
            PricingTier {
                tier_name: "Enterprise".into(),
                monthly_price: 299900, // $2,999.00
                usage_limits: HashMap::from([(
                    "api_calls".into(),
                    UsageLimit {
                        included_quantity: 1000000,
                        unit: UsageUnit::Count,
                        hard_limit: None,
                    },
                )]),
                overage_rates: HashMap::from([
                    ("api_calls".into(), 2), // $0.0002 per call
                    ("transactions".into(), 20), // $0.002 per transaction
                ]),
            },
        );

        Self {
            pricing_tiers,
            records: Vec::new(),
        }
    }

    /// Record usage for a tenant.
    pub fn record_usage(
        &mut self,
        tenant_id: Uuid,
        usage_type: UsageType,
        quantity: u64,
        metadata: Option<serde_json::Value>,
    ) {
        let record = UsageRecord {
            record_id: Uuid::now_v7(),
            tenant_id,
            usage_type: usage_type.clone(),
            quantity,
            unit: UsageUnit::Count,
            timestamp: Utc::now(),
            metadata,
        };

        self.records.push(record);
    }

    /// Calculate usage for a tenant in a billing period.
    pub fn calculate_usage(
        &self,
        tenant_id: Uuid,
        plan: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> TenantUsage {
        let tier = self
            .pricing_tiers
            .get(plan)
            .expect("Unknown pricing tier");

        // Aggregate usage by type
        let mut usage_by_type: HashMap<String, UsageAggregation> = HashMap::new();

        for record in &self.records {
            if record.tenant_id != tenant_id {
                continue;
            }
            if record.timestamp < period_start || record.timestamp > period_end {
                continue;
            }

            let usage_type_key = record.usage_type.as_str().to_string();
            let aggregation = usage_by_type
                .entry(usage_type_key.clone())
                .or_insert_with(|| UsageAggregation {
                    usage_type: usage_type_key.clone(),
                    total_quantity: 0,
                    included_quantity: tier
                        .usage_limits
                        .get(&usage_type_key)
                        .map(|l| l.included_quantity)
                        .unwrap_or(0),
                    billable_quantity: 0,
                    unit_price: tier
                        .overage_rates
                        .get(&usage_type_key)
                        .copied()
                        .unwrap_or(0),
                    total_cost: 0,
                });

            aggregation.total_quantity += record.quantity;
        }

        // Calculate billable quantities and costs
        let mut total_cost = tier.monthly_price;

        for aggregation in usage_by_type.values_mut() {
            aggregation.billable_quantity = aggregation
                .total_quantity
                .saturating_sub(aggregation.included_quantity);
            aggregation.total_cost = aggregation.billable_quantity as i64 * aggregation.unit_price;
            total_cost += aggregation.total_cost;
        }

        TenantUsage {
            tenant_id,
            billing_period_start: period_start,
            billing_period_end: period_end,
            usage_by_type,
            total_estimated_cost: total_cost,
        }
    }

    /// Check if a tenant is approaching their usage limits.
    pub fn check_usage_alerts(
        &self,
        tenant_id: Uuid,
        plan: &str,
    ) -> Vec<UsageAlert> {
        let tier = self
            .pricing_tiers
            .get(plan)
            .expect("Unknown pricing tier");

        let mut alerts = Vec::new();
        let now = Utc::now();
        let period_start = now - chrono::Duration::days(30);

        // Count current period usage
        let mut usage_counts: HashMap<String, u64> = HashMap::new();
        for record in &self.records {
            if record.tenant_id != tenant_id {
                continue;
            }
            if record.timestamp < period_start {
                continue;
            }
            *usage_counts
                .entry(record.usage_type.as_str().to_string())
                .or_insert(0) += record.quantity;
        }

        // Check against limits
        for (usage_type, limit) in &tier.usage_limits {
            if let Some(current) = usage_counts.get(usage_type) {
                let percentage = (*current as f64 / limit.included_quantity as f64) * 100.0;

                if percentage >= 90.0 {
                    alerts.push(UsageAlert {
                        alert_type: UsageAlertType::ApproachingLimit,
                        usage_type: usage_type.clone(),
                        current_usage: *current,
                        limit: limit.included_quantity,
                        percentage,
                        message: format!(
                            "You've used {:.0}% of your {} limit",
                            percentage, usage_type
                        ),
                    });
                }

                if let Some(hard_limit) = limit.hard_limit {
                    if *current >= hard_limit {
                        alerts.push(UsageAlert {
                            alert_type: UsageAlertType::LimitExceeded,
                            usage_type: usage_type.clone(),
                            current_usage: *current,
                            limit: hard_limit,
                            percentage: 100.0,
                            message: format!(
                                "You've exceeded your {} hard limit",
                                usage_type
                            ),
                        });
                    }
                }
            }
        }

        alerts
    }
}

impl Default for UsageMeteringService {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Alert Types ─────────────────────────────────────────────────────────────

/// Usage alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAlert {
    pub alert_type: UsageAlertType,
    pub usage_type: String,
    pub current_usage: u64,
    pub limit: u64,
    pub percentage: f64,
    pub message: String,
}

/// Type of usage alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsageAlertType {
    ApproachingLimit,
    LimitExceeded,
    OverageWarning,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_metering() {
        let mut service = UsageMeteringService::new();
        let tenant_id = Uuid::now_v7();

        // Record some usage
        service.record_usage(tenant_id, UsageType::ApiCalls, 1000, None);
        service.record_usage(tenant_id, UsageType::Transactions, 500, None);

        let now = Utc::now();
        let period_start = now - chrono::Duration::days(30);

        let usage = service.calculate_usage(tenant_id, "starter", period_start, now);

        assert_eq!(usage.tenant_id, tenant_id);
        assert!(usage.total_estimated_cost > 0);

        // Starter plan includes 10,000 API calls, so 1,000 should be free
        let api_calls = usage.usage_by_type.get("api_calls").unwrap();
        assert_eq!(api_calls.billable_quantity, 0);
    }

    #[test]
    fn test_usage_alerts() {
        let mut service = UsageMeteringService::new();
        let tenant_id = Uuid::now_v7();

        // Record usage approaching limit
        service.record_usage(tenant_id, UsageType::ApiCalls, 9500, None);

        let alerts = service.check_usage_alerts(tenant_id, "starter");
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].alert_type, UsageAlertType::ApproachingLimit);
    }
}
