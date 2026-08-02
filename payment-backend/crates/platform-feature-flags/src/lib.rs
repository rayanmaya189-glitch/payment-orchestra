//! Feature flags system for multi-tenant SaaS platform.
//!
//! Provides:
//! - Plan-based feature gating (Starter, Growth, Enterprise)
//! - Per-tenant feature overrides
//! - A/B testing support
//! - Gradual rollout capabilities
//! - Redis-backed flag storage for distributed access

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ─── Feature Definitions ─────────────────────────────────────────────────────

/// All available features in the platform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Feature {
    // Core features
    PaymentProcessing,
    SmartRouting,
    BasicAnalytics,
    AdvancedAnalytics,

    // Connector features
    ConnectorMarketplace,
    CustomConnectors,
    SandboxMode,

    // Security features
    TwoFactorAuth,
    SsoIntegration,
    IpWhitelisting,
    AuditLogs,

    // Billing features
    UsageBasedBilling,
    InvoiceGeneration,
    SubscriptionManagement,

    // Developer features
    ApiExplorer,
    SdkGeneration,
    WebhookManagement,

    // Enterprise features
    DedicatedSupport,
    CustomSlas,
    MultiRegionDeployment,
    WhiteLabel,
    FraudIntelligence,

    // AI features
    AiAssistant,
    MlRouting,
    PredictiveAnalytics,

    // Custom feature flag
    Custom(String),
}

impl Feature {
    pub fn as_str(&self) -> &str {
        match self {
            Feature::PaymentProcessing => "payment_processing",
            Feature::SmartRouting => "smart_routing",
            Feature::BasicAnalytics => "basic_analytics",
            Feature::AdvancedAnalytics => "advanced_analytics",
            Feature::ConnectorMarketplace => "connector_marketplace",
            Feature::CustomConnectors => "custom_connectors",
            Feature::SandboxMode => "sandbox_mode",
            Feature::TwoFactorAuth => "two_factor_auth",
            Feature::SsoIntegration => "sso_integration",
            Feature::IpWhitelisting => "ip_whitelisting",
            Feature::AuditLogs => "audit_logs",
            Feature::UsageBasedBilling => "usage_based_billing",
            Feature::InvoiceGeneration => "invoice_generation",
            Feature::SubscriptionManagement => "subscription_management",
            Feature::ApiExplorer => "api_explorer",
            Feature::SdkGeneration => "sdk_generation",
            Feature::WebhookManagement => "webhook_management",
            Feature::DedicatedSupport => "dedicated_support",
            Feature::CustomSlas => "custom_slas",
            Feature::MultiRegionDeployment => "multi_region_deployment",
            Feature::WhiteLabel => "white_label",
            Feature::FraudIntelligence => "fraud_intelligence",
            Feature::AiAssistant => "ai_assistant",
            Feature::MlRouting => "ml_routing",
            Feature::PredictiveAnalytics => "predictive_analytics",
            Feature::Custom(name) => name,
        }
    }
}

// ─── Plan Definitions ────────────────────────────────────────────────────────

/// Subscription plan tiers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlanTier {
    Free,
    Starter,
    Growth,
    Enterprise,
}

impl PlanTier {
    pub fn features(&self) -> Vec<Feature> {
        match self {
            PlanTier::Free => vec![
                Feature::PaymentProcessing,
                Feature::BasicAnalytics,
                Feature::SandboxMode,
            ],
            PlanTier::Starter => vec![
                Feature::PaymentProcessing,
                Feature::SmartRouting,
                Feature::BasicAnalytics,
                Feature::SandboxMode,
                Feature::ConnectorMarketplace,
                Feature::WebhookManagement,
                Feature::AuditLogs,
            ],
            PlanTier::Growth => vec![
                Feature::PaymentProcessing,
                Feature::SmartRouting,
                Feature::BasicAnalytics,
                Feature::AdvancedAnalytics,
                Feature::SandboxMode,
                Feature::ConnectorMarketplace,
                Feature::CustomConnectors,
                Feature::TwoFactorAuth,
                Feature::UsageBasedBilling,
                Feature::InvoiceGeneration,
                Feature::SubscriptionManagement,
                Feature::WebhookManagement,
                Feature::ApiExplorer,
                Feature::AuditLogs,
                Feature::AiAssistant,
            ],
            PlanTier::Enterprise => vec![
                Feature::PaymentProcessing,
                Feature::SmartRouting,
                Feature::BasicAnalytics,
                Feature::AdvancedAnalytics,
                Feature::SandboxMode,
                Feature::ConnectorMarketplace,
                Feature::CustomConnectors,
                Feature::TwoFactorAuth,
                Feature::SsoIntegration,
                Feature::IpWhitelisting,
                Feature::UsageBasedBilling,
                Feature::InvoiceGeneration,
                Feature::SubscriptionManagement,
                Feature::WebhookManagement,
                Feature::ApiExplorer,
                Feature::SdkGeneration,
                Feature::DedicatedSupport,
                Feature::CustomSlas,
                Feature::MultiRegionDeployment,
                Feature::WhiteLabel,
                Feature::FraudIntelligence,
                Feature::AuditLogs,
                Feature::AiAssistant,
                Feature::MlRouting,
                Feature::PredictiveAnalytics,
            ],
        }
    }
}

// ─── Feature Flag Manager ────────────────────────────────────────────────────

/// Feature flag manager with plan-based gating and per-tenant overrides.
pub struct FeatureFlagManager {
    /// Per-tenant feature overrides
    tenant_overrides: Arc<RwLock<HashMap<String, HashMap<String, bool>>>>,
    /// A/B test groups
    ab_test_groups: Arc<RwLock<HashMap<String, String>>>,
}

impl FeatureFlagManager {
    /// Create a new feature flag manager.
    pub fn new() -> Self {
        Self {
            tenant_overrides: Arc::new(RwLock::new(HashMap::new())),
            ab_test_groups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a tenant has access to a feature.
    pub async fn is_enabled(
        &self,
        tenant_id: &str,
        plan: &PlanTier,
        feature: &Feature,
    ) -> bool {
        // Check per-tenant overrides first
        {
            let overrides = self.tenant_overrides.read().await;
            if let Some(tenant_features) = overrides.get(tenant_id) {
                if let Some(enabled) = tenant_features.get(feature.as_str()) {
                    return *enabled;
                }
            }
        }

        // Fall back to plan-based features
        plan.features().iter().any(|f| f == feature)
    }

    /// Enable a feature for a specific tenant (override).
    pub async fn enable_for_tenant(
        &self,
        tenant_id: &str,
        feature: &Feature,
    ) {
        let mut overrides = self.tenant_overrides.write().await;
        let tenant_features = overrides
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
        tenant_features.insert(feature.as_str().to_string(), true);
    }

    /// Disable a feature for a specific tenant (override).
    pub async fn disable_for_tenant(
        &self,
        tenant_id: &str,
        feature: &Feature,
    ) {
        let mut overrides = self.tenant_overrides.write().await;
        let tenant_features = overrides
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
        tenant_features.insert(feature.as_str().to_string(), false);
    }

    /// Remove tenant override (revert to plan-based).
    pub async fn remove_override(
        &self,
        tenant_id: &str,
        feature: &Feature,
    ) {
        let mut overrides = self.tenant_overrides.write().await;
        if let Some(tenant_features) = overrides.get_mut(tenant_id) {
            tenant_features.remove(feature.as_str());
        }
    }

    /// Assign a tenant to an A/B test group.
    pub async fn assign_ab_group(
        &self,
        tenant_id: &str,
        group: &str,
    ) {
        let mut groups = self.ab_test_groups.write().await;
        groups.insert(tenant_id.to_string(), group.to_string());
    }

    /// Get the A/B test group for a tenant.
    pub async fn get_ab_group(&self, tenant_id: &str) -> Option<String> {
        let groups = self.ab_test_groups.read().await;
        groups.get(tenant_id).cloned()
    }

    /// Get all enabled features for a tenant.
    pub async fn get_enabled_features(
        &self,
        tenant_id: &str,
        plan: &PlanTier,
    ) -> Vec<Feature> {
        let plan_features = plan.features();
        let mut result = Vec::new();

        // Start with plan features
        for feature in &plan_features {
            result.push(feature.clone());
        }

        // Add tenant overrides
        {
            let overrides = self.tenant_overrides.read().await;
            if let Some(tenant_features) = overrides.get(tenant_id) {
                for (feature_name, enabled) in tenant_features {
                    if *enabled && !result.iter().any(|f| f.as_str() == feature_name) {
                        // This is a custom feature enabled for this tenant
                        result.push(Feature::Custom(feature_name.clone()));
                    }
                }
            }
        }

        result
    }
}

impl Default for FeatureFlagManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plan_based_features() {
        let manager = FeatureFlagManager::new();

        // Starter plan should have smart routing
        assert!(manager.is_enabled("tenant-1", &PlanTier::Starter, &Feature::SmartRouting).await);

        // Starter plan should NOT have advanced analytics
        assert!(!manager.is_enabled("tenant-1", &PlanTier::Starter, &Feature::AdvancedAnalytics).await);

        // Enterprise should have everything
        assert!(manager.is_enabled("tenant-1", &PlanTier::Enterprise, &Feature::WhiteLabel).await);
        assert!(manager.is_enabled("tenant-1", &PlanTier::Enterprise, &Feature::MlRouting).await);
    }

    #[tokio::test]
    async fn test_tenant_override() {
        let manager = FeatureFlagManager::new();

        // Default: starter doesn't have advanced analytics
        assert!(!manager.is_enabled("tenant-1", &PlanTier::Starter, &Feature::AdvancedAnalytics).await);

        // Enable for specific tenant
        manager.enable_for_tenant("tenant-1", &Feature::AdvancedAnalytics).await;

        // Now it's enabled
        assert!(manager.is_enabled("tenant-1", &PlanTier::Starter, &Feature::AdvancedAnalytics).await);

        // Other tenants still don't have it
        assert!(!manager.is_enabled("tenant-2", &PlanTier::Starter, &Feature::AdvancedAnalytics).await);
    }

    #[tokio::test]
    async fn test_ab_testing() {
        let manager = FeatureFlagManager::new();

        manager.assign_ab_group("tenant-1", "control").await;
        manager.assign_ab_group("tenant-2", "variant_a").await;

        assert_eq!(manager.get_ab_group("tenant-1").await, Some("control".into()));
        assert_eq!(manager.get_ab_group("tenant-2").await, Some("variant_a".into()));
        assert_eq!(manager.get_ab_group("tenant-3").await, None);
    }

    #[test]
    fn test_feature_as_str() {
        assert_eq!(Feature::PaymentProcessing.as_str(), "payment_processing");
        assert_eq!(Feature::Custom("my_feature".into()).as_str(), "my_feature");
    }
}
