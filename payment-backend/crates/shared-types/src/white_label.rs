//! White-label platform types — customization for payment facilitators.
//!
//! Provides:
//! - Brand customization (logo, colors, domain)
//! - Feature flagging per white-label partner
//! - Revenue sharing configuration
//! - Custom domain support

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── White Label Configuration ───────────────────────────────────────────────

/// White-label partner configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteLabelConfig {
    pub config_id: Uuid,
    pub partner_id: Uuid,
    pub partner_name: String,
    pub branding: BrandingConfig,
    pub domain_config: DomainConfig,
    pub feature_flags: FeatureFlags,
    pub revenue_share: RevenueShareConfig,
    pub status: WhiteLabelStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status of white-label configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WhiteLabelStatus {
    Active,
    Suspended,
    PendingSetup,
}

// ─── Branding ────────────────────────────────────────────────────────────────

/// Brand customization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub primary_color: String,   // Hex color
    pub secondary_color: String, // Hex color
    pub accent_color: String,    // Hex color
    pub background_color: String,
    pub text_color: String,
    pub font_family: Option<String>,
    pub custom_css: Option<String>,
    pub email_template: Option<EmailTemplate>,
}

/// Email template customization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    pub header_html: Option<String>,
    pub footer_html: Option<String>,
    pub support_email: Option<String>,
    pub from_name: String,
    pub from_email: String,
}

// ─── Domain Configuration ────────────────────────────────────────────────────

/// Custom domain configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    pub custom_domain: Option<String>,
    pub subdomain: Option<String>,
    pub ssl_status: SslStatus,
    pub ssl_issued_at: Option<DateTime<Utc>>,
    pub ssl_expires_at: Option<DateTime<Utc>>,
}

/// SSL certificate status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SslStatus {
    Active,
    Pending,
    Expired,
    Failed,
}

// ─── Feature Flags ───────────────────────────────────────────────────────────

/// Feature flags for white-label partners.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub payment_processing: bool,
    pub smart_routing: bool,
    pub analytics: bool,
    pub reconciliation: bool,
    pub dispute_management: bool,
    pub subscription_billing: bool,
    pub ai_assistant: bool,
    pub white_label_dashboard: bool,
    pub custom_webhooks: bool,
    pub api_access: bool,
    pub sandbox_mode: bool,
    pub multi_currency: bool,
    pub advanced_fraud: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            payment_processing: true,
            smart_routing: false,
            analytics: true,
            reconciliation: false,
            dispute_management: false,
            subscription_billing: false,
            ai_assistant: false,
            white_label_dashboard: true,
            custom_webhooks: true,
            api_access: true,
            sandbox_mode: true,
            multi_currency: false,
            advanced_fraud: false,
        }
    }
}

// ─── Revenue Share ───────────────────────────────────────────────────────────

/// Revenue sharing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueShareConfig {
    /// Revenue share percentage for the partner (0-100)
    pub partner_percentage: f64,
    /// Platform percentage (100 - partner_percentage)
    pub platform_percentage: f64,
    /// Minimum monthly guarantee
    pub minimum_monthly_guarantee: i64,
    /// Payment terms (net 30, net 60, etc.)
    pub payment_terms_days: u32,
    /// Revenue share tier thresholds
    pub tiers: Vec<RevenueShareTier>,
}

/// Revenue share tier based on volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueShareTier {
    pub min_volume: i64,
    pub max_volume: Option<i64>,
    pub partner_percentage: f64,
}

// ─── White Label Service ─────────────────────────────────────────────────────

/// Service for managing white-label configurations.
pub struct WhiteLabelService {
    configs: Vec<WhiteLabelConfig>,
}

impl WhiteLabelService {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
        }
    }

    /// Create a new white-label configuration.
    pub fn create_config(
        &mut self,
        partner_id: Uuid,
        partner_name: String,
    ) -> WhiteLabelConfig {
        let config = WhiteLabelConfig {
            config_id: Uuid::now_v7(),
            partner_id,
            partner_name,
            branding: BrandingConfig {
                logo_url: None,
                favicon_url: None,
                primary_color: "#0ea5e9".into(),
                secondary_color: "#0284c7".into(),
                accent_color: "#f59e0b".into(),
                background_color: "#ffffff".into(),
                text_color: "#111827".into(),
                font_family: None,
                custom_css: None,
                email_template: None,
            },
            domain_config: DomainConfig {
                custom_domain: None,
                subdomain: None,
                ssl_status: SslStatus::Pending,
                ssl_issued_at: None,
                ssl_expires_at: None,
            },
            feature_flags: FeatureFlags::default(),
            revenue_share: RevenueShareConfig {
                partner_percentage: 30.0,
                platform_percentage: 70.0,
                minimum_monthly_guarantee: 0,
                payment_terms_days: 30,
                tiers: vec![],
            },
            status: WhiteLabelStatus::PendingSetup,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.configs.push(config.clone());
        config
    }

    /// Get configuration by partner ID.
    pub fn get_by_partner(&self, partner_id: Uuid) -> Option<&WhiteLabelConfig> {
        self.configs.iter().find(|c| c.partner_id == partner_id)
    }

    /// Update branding configuration.
    pub fn update_branding(
        &mut self,
        config_id: Uuid,
        branding: BrandingConfig,
    ) -> Result<(), String> {
        let config = self
            .configs
            .iter_mut()
            .find(|c| c.config_id == config_id)
            .ok_or("Config not found")?;

        config.branding = branding;
        config.updated_at = Utc::now();
        Ok(())
    }

    /// Get all active configurations.
    pub fn list_active(&self) -> Vec<&WhiteLabelConfig> {
        self.configs
            .iter()
            .filter(|c| c.status == WhiteLabelStatus::Active)
            .collect()
    }
}

impl Default for WhiteLabelService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_white_label_config() {
        let mut service = WhiteLabelService::new();
        let config = service.create_config(Uuid::nil(), "Test Partner".into());

        assert_eq!(config.partner_name, "Test Partner");
        assert_eq!(config.status, WhiteLabelStatus::PendingSetup);
        assert_eq!(config.revenue_share.partner_percentage, 30.0);
    }

    #[test]
    fn test_default_feature_flags() {
        let flags = FeatureFlags::default();
        assert!(flags.payment_processing);
        assert!(flags.analytics);
        assert!(!flags.smart_routing);
        assert!(!flags.advanced_fraud);
    }
}
