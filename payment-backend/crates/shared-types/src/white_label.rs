//! White-label configuration types for platform customization.
//!
//! This module provides:
//! - Custom branding (logo, colors, fonts)
//! - Custom domain configuration
//! - Email template customization
//! - Checkout page customization
//! - API response customization
//! - Feature flags per tenant

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use thiserror::Error;

// ---------------------------------------------------------------------------
// White-Label Configuration
// ---------------------------------------------------------------------------

/// White-label configuration for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteLabelConfig {
    pub config_id: Uuid,
    pub operator_id: Uuid,
    pub branding: BrandingConfig,
    pub domains: DomainConfig,
    pub email: EmailConfig,
    pub checkout: CheckoutConfig,
    pub api: ApiCustomConfig,
    pub features: FeatureFlags,
    pub status: WhiteLabelStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// White-label status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WhiteLabelStatus {
    /// White-label is not enabled
    Disabled,
    /// White-label is being configured
    Configuring,
    /// White-label is active
    Active,
    /// White-label is suspended
    Suspended,
}

impl std::fmt::Display for WhiteLabelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => write!(f, "disabled"),
            Self::Configuring => write!(f, "configuring"),
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

// ---------------------------------------------------------------------------
// Branding Configuration
// ---------------------------------------------------------------------------

/// Branding configuration for visual customization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Company/tenant name
    pub company_name: String,
    /// Logo URL (SVG recommended)
    pub logo_url: Option<String>,
    /// Logo alt text
    pub logo_alt: Option<String>,
    /// Favicon URL
    pub favicon_url: Option<String>,
    /// Primary brand color (hex)
    pub primary_color: String,
    /// Secondary brand color (hex)
    pub secondary_color: Option<String>,
    /// Accent color (hex)
    pub accent_color: Option<String>,
    /// Background color (hex)
    pub background_color: Option<String>,
    /// Text color (hex)
    pub text_color: Option<String>,
    /// Font family
    pub font_family: Option<String>,
    /// Custom CSS (injected into pages)
    pub custom_css: Option<String>,
    /// Footer text
    pub footer_text: Option<String>,
    /// Support email
    pub support_email: Option<String>,
    /// Support phone
    pub support_phone: Option<String>,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            company_name: "PaymentOrchestra".to_string(),
            logo_url: None,
            logo_alt: None,
            favicon_url: None,
            primary_color: "#0ea5e9".to_string(),
            secondary_color: None,
            accent_color: None,
            background_color: None,
            text_color: None,
            font_family: None,
            custom_css: None,
            footer_text: None,
            support_email: None,
            support_phone: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Domain Configuration
// ---------------------------------------------------------------------------

/// Custom domain configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    /// Primary custom domain (e.g., pay.yourcompany.com)
    pub primary_domain: Option<String>,
    /// Custom API domain (e.g., api.yourcompany.com)
    pub api_domain: Option<String>,
    /// Custom checkout domain (e.g., checkout.yourcompany.com)
    pub checkout_domain: Option<String>,
    /// SSL certificate status
    pub ssl_status: SslStatus,
    /// DNS verification status
    pub dns_verified: bool,
    /// Domain verification token
    pub verification_token: Option<String>,
}

/// SSL certificate status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SslStatus {
    /// SSL not configured
    NotConfigured,
    /// SSL certificate pending
    Pending,
    /// SSL certificate active
    Active,
    /// SSL certificate expired
    Expired,
    /// SSL certificate error
    Error(String),
}

// ---------------------------------------------------------------------------
// Email Configuration
// ---------------------------------------------------------------------------

/// Email template configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// From name for emails
    pub from_name: String,
    /// From email address
    pub from_email: String,
    /// Reply-to email address
    pub reply_to: Option<String>,
    /// Custom email templates
    pub templates: Vec<EmailTemplate>,
    /// Email header logo URL
    pub header_logo_url: Option<String>,
    /// Email footer text
    pub footer_text: Option<String>,
    /// Email theme
    pub theme: EmailTheme,
}

/// Email template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    /// Template ID
    pub template_id: Uuid,
    /// Template name (e.g., "payment_receipt", "refund_notification")
    pub name: String,
    /// Template subject line
    pub subject: String,
    /// HTML body template (with Handlebars placeholders)
    pub html_body: String,
    /// Plain text body template
    pub text_body: Option<String>,
    /// Is this template enabled?
    pub enabled: bool,
}

/// Email theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailTheme {
    /// Default theme
    Default,
    /// Minimal theme
    Minimal,
    /// Professional theme
    Professional,
    /// Custom theme (uses custom_css)
    Custom,
}

// ---------------------------------------------------------------------------
// Checkout Configuration
// ---------------------------------------------------------------------------

/// Checkout page customization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutConfig {
    /// Checkout page title
    pub title: Option<String>,
    /// Checkout page description
    pub description: Option<String>,
    /// Custom logo for checkout
    pub logo_url: Option<String>,
    /// Background color
    pub background_color: Option<String>,
    /// Button color
    pub button_color: Option<String>,
    /// Button text color
    pub button_text_color: Option<String>,
    /// Border radius (px)
    pub border_radius: Option<u32>,
    /// Custom CSS for checkout
    pub custom_css: Option<String>,
    /// Show company name in footer
    pub show_footer: bool,
    /// Redirect URL after successful payment
    pub success_redirect_url: Option<String>,
    /// Redirect URL after failed payment
    pub failure_redirect_url: Option<String>,
    /// Collect billing address
    pub collect_billing_address: bool,
    /// Collect shipping address
    pub collect_shipping_address: bool,
    /// Show order summary
    pub show_order_summary: bool,
    /// Supported payment methods
    pub payment_methods: Vec<String>,
}

impl Default for CheckoutConfig {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            logo_url: None,
            background_color: None,
            button_color: None,
            button_text_color: None,
            border_radius: Some(8),
            custom_css: None,
            show_footer: true,
            success_redirect_url: None,
            failure_redirect_url: None,
            collect_billing_address: false,
            collect_shipping_address: false,
            show_order_summary: true,
            payment_methods: vec![
                "card".to_string(),
                "bank_transfer".to_string(),
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// API Custom Configuration
// ---------------------------------------------------------------------------

/// API response customization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCustomConfig {
    /// Custom API response wrapper
    pub response_wrapper: Option<String>,
    /// Custom error format
    pub error_format: Option<String>,
    /// Include tenant ID in responses
    pub include_tenant_id: bool,
    /// Custom header prefix
    pub header_prefix: Option<String>,
    /// API version in responses
    pub include_api_version: bool,
    /// Custom webhook URL
    pub webhook_url: Option<String>,
    /// Webhook signing secret
    pub webhook_secret: Option<String>,
}

// ---------------------------------------------------------------------------
// Feature Flags
// ---------------------------------------------------------------------------

/// Feature flags for tenant-specific features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Smart routing enabled
    pub smart_routing: bool,
    /// Network tokenization enabled
    pub network_tokenization: bool,
    /// Advanced analytics enabled
    pub advanced_analytics: bool,
    /// AI assistant enabled
    pub ai_assistant: bool,
    /// Custom connectors enabled
    pub custom_connectors: bool,
    /// Multi-currency enabled
    pub multi_currency: bool,
    /// Subscription billing enabled
    pub subscription_billing: bool,
    /// Invoice generation enabled
    pub invoice_generation: bool,
    /// Reconciliation enabled
    pub reconciliation: bool,
    /// Dispute management enabled
    pub dispute_management: bool,
    /// Priority support enabled
    pub priority_support: bool,
    /// Dedicated infrastructure enabled
    pub dedicated_infrastructure: bool,
    /// Custom feature flags
    pub custom: std::collections::HashMap<String, bool>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            smart_routing: true,
            network_tokenization: false,
            advanced_analytics: false,
            ai_assistant: false,
            custom_connectors: false,
            multi_currency: true,
            subscription_billing: false,
            invoice_generation: false,
            reconciliation: false,
            dispute_management: false,
            priority_support: false,
            dedicated_infrastructure: false,
            custom: std::collections::HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// White-Label Service Trait
// ---------------------------------------------------------------------------

/// White-label configuration service trait.
#[async_trait::async_trait]
pub trait WhiteLabelService: Send + Sync {
    /// Get white-label configuration for an operator.
    async fn get_config(
        &self,
        operator_id: Uuid,
    ) -> Result<WhiteLabelConfig, WhiteLabelError>;

    /// Update white-label configuration.
    async fn update_config(
        &self,
        operator_id: Uuid,
        config: WhiteLabelConfig,
    ) -> Result<WhiteLabelConfig, WhiteLabelError>;

    /// Enable white-label for an operator.
    async fn enable(
        &self,
        operator_id: Uuid,
    ) -> Result<WhiteLabelConfig, WhiteLabelError>;

    /// Disable white-label for an operator.
    async fn disable(
        &self,
        operator_id: Uuid,
    ) -> Result<(), WhiteLabelError>;

    /// Verify custom domain.
    async fn verify_domain(
        &self,
        operator_id: Uuid,
        domain: &str,
    ) -> Result<DomainVerificationResult, WhiteLabelError>;

    /// Get resolved branding for rendering.
    async fn get_branding(
        &self,
        operator_id: Uuid,
    ) -> Result<BrandingConfig, WhiteLabelError>;

    /// Check if a feature is enabled for an operator.
    async fn is_feature_enabled(
        &self,
        operator_id: Uuid,
        feature: &str,
    ) -> Result<bool, WhiteLabelError>;
}

/// Domain verification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainVerificationResult {
    pub verified: bool,
    pub dns_records: Vec<DnsRecord>,
    pub ssl_ready: bool,
    pub errors: Vec<String>,
}

/// DNS record for domain verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub record_type: String,
    pub name: String,
    pub value: String,
    pub verified: bool,
}

/// White-label errors.
#[derive(Debug, Clone, Error)]
pub enum WhiteLabelError {
    #[error("White-label not configured for this operator")]
    NotConfigured,
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Domain verification failed: {0}")]
    DomainVerificationFailed(String),
    #[error("SSL certificate error: {0}")]
    SslError(String),
    #[error("Feature not available: {0}")]
    FeatureNotAvailable(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branding_config_default() {
        let config = BrandingConfig::default();
        assert_eq!(config.company_name, "PaymentOrchestra");
        assert_eq!(config.primary_color, "#0ea5e9");
    }

    #[test]
    fn test_feature_flags_default() {
        let flags = FeatureFlags::default();
        assert!(flags.smart_routing);
        assert!(!flags.advanced_analytics);
        assert!(!flags.dedicated_infrastructure);
    }

    #[test]
    fn test_checkout_config_default() {
        let config = CheckoutConfig::default();
        assert_eq!(config.border_radius, Some(8));
        assert!(config.show_footer);
        assert!(config.payment_methods.contains(&"card".to_string()));
    }
}
