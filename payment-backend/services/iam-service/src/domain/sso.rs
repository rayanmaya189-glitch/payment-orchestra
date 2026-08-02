//! SSO/SAML support for Identity & Access Management.
//!
//! This module provides:
//! - SAML 2.0 identity provider configuration
//! - OAuth2/OIDC integration
//! - SSO session management
//! - Identity provider metadata

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SSO Provider type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SsoProviderType {
    Saml2,
    Oidc,
    OAuth2,
}

/// SSO Provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProvider {
    pub provider_id: Uuid,
    pub name: String,
    pub provider_type: SsoProviderType,
    pub issuer_url: String,
    pub sso_url: String,
    pub certificate: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Vec<String>,
    pub attribute_mapping: AttributeMapping,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Attribute mapping from SSO provider to platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeMapping {
    /// Field mapping for email
    pub email: String,
    /// Field mapping for first name
    pub first_name: Option<String>,
    /// Field mapping for last name
    pub last_name: Option<String>,
    /// Field mapping for user ID
    pub user_id: Option<String>,
    /// Field mapping for groups/roles
    pub groups: Option<String>,
}

impl Default for AttributeMapping {
    fn default() -> Self {
        Self {
            email: "email".into(),
            first_name: Some("given_name".into()),
            last_name: Some("family_name".into()),
            user_id: Some("sub".into()),
            groups: Some("groups".into()),
        }
    }
}

/// SSO Session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    pub session_id: Uuid,
    pub principal_id: Uuid,
    pub provider_id: Uuid,
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl SsoSession {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// SAML Response attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlResponse {
    pub name_id: String,
    pub session_index: String,
    pub attributes: std::collections::HashMap<String, Vec<String>>,
    pub issuer: String,
    pub not_before: Option<DateTime<Utc>>,
    pub not_on_or_after: Option<DateTime<Utc>>,
}

/// OIDC User Info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
}

/// SSO Manager for handling SSO operations.
pub struct SsoManager {
    providers: std::collections::HashMap<Uuid, SsoProvider>,
}

impl SsoManager {
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    /// Register a new SSO provider.
    pub fn register_provider(&mut self, provider: SsoProvider) {
        self.providers.insert(provider.provider_id, provider);
    }

    /// Get SSO provider by ID.
    pub fn get_provider(&self, provider_id: &Uuid) -> Option<&SsoProvider> {
        self.providers.get(provider_id)
    }

    /// List all enabled providers.
    pub fn list_enabled_providers(&self) -> Vec<&SsoProvider> {
        self.providers.values().filter(|p| p.enabled).collect()
    }

    /// Find provider by issuer URL.
    pub fn find_by_issuer(&self, issuer_url: &str) -> Option<&SsoProvider> {
        self.providers.values().find(|p| p.issuer_url == issuer_url)
    }

    /// Validate SAML response.
    pub fn validate_saml_response(
        &self,
        response: &SamlResponse,
        provider_id: &Uuid,
    ) -> Result<(), SsoError> {
        let provider = self
            .providers
            .get(provider_id)
            .ok_or(SsoError::ProviderNotFound)?;

        if !provider.enabled {
            return Err(SsoError::ProviderDisabled);
        }

        if response.issuer != provider.issuer_url {
            return Err(SsoError::InvalidIssuer);
        }

        // Check time validity
        if let Some(not_before) = response.not_before {
            if Utc::now() < not_before {
                return Err(SsoError::TokenNotYetValid);
            }
        }

        if let Some(not_on_or_after) = response.not_on_or_after {
            if Utc::now() >= not_on_or_after {
                return Err(SsoError::TokenExpired);
            }
        }

        Ok(())
    }

    /// Extract user info from SAML response.
    pub fn extract_user_from_saml(
        &self,
        response: &SamlResponse,
        provider_id: &Uuid,
    ) -> Result<SsoUserInfo, SsoError> {
        let provider = self
            .providers
            .get(provider_id)
            .ok_or(SsoError::ProviderNotFound)?;

        let email = response
            .attributes
            .get(&provider.attribute_mapping.email)
            .and_then(|v| v.first().cloned())
            .ok_or(SsoError::MissingAttribute("email".into()))?;

        let first_name = provider
            .attribute_mapping
            .first_name
            .as_ref()
            .and_then(|field| response.attributes.get(field).and_then(|v| v.first().cloned()));

        let last_name = provider
            .attribute_mapping
            .last_name
            .as_ref()
            .and_then(|field| response.attributes.get(field).and_then(|v| v.first().cloned()));

        let groups = provider
            .attribute_mapping
            .groups
            .as_ref()
            .and_then(|field| response.attributes.get(field).cloned())
            .unwrap_or_default();

        Ok(SsoUserInfo {
            email,
            first_name,
            last_name,
            groups,
            external_id: response.name_id.clone(),
        })
    }
}

impl Default for SsoManager {
    fn default() -> Self {
        Self::new()
    }
}

/// SSO User Info extracted from provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoUserInfo {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub groups: Vec<String>,
    pub external_id: String,
}

/// SSO errors.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum SsoError {
    #[error("SSO provider not found")]
    ProviderNotFound,

    #[error("SSO provider is disabled")]
    ProviderDisabled,

    #[error("Invalid issuer")]
    InvalidIssuer,

    #[error("Token not yet valid")]
    TokenNotYetValid,

    #[error("Token expired")]
    TokenExpired,

    #[error("Missing attribute: {0}")]
    MissingAttribute(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_provider() -> SsoProvider {
        SsoProvider {
            provider_id: Uuid::now_v7(),
            name: "Test SAML Provider".into(),
            provider_type: SsoProviderType::Saml2,
            issuer_url: "https://idp.example.com".into(),
            sso_url: "https://idp.example.com/sso".into(),
            certificate: None,
            client_id: None,
            client_secret: None,
            scopes: vec!["openid".into(), "email".into()],
            attribute_mapping: AttributeMapping::default(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_sso_manager_register_provider() {
        let mut manager = SsoManager::new();
        let provider = create_test_provider();
        let provider_id = provider.provider_id;

        manager.register_provider(provider);

        let retrieved = manager.get_provider(&provider_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test SAML Provider");
    }

    #[test]
    fn test_sso_manager_list_enabled_providers() {
        let mut manager = SsoManager::new();
        let provider1 = create_test_provider();
        let mut provider2 = create_test_provider();
        provider2.enabled = false;

        manager.register_provider(provider1);
        manager.register_provider(provider2);

        let enabled = manager.list_enabled_providers();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_validate_saml_response_success() {
        let mut manager = SsoManager::new();
        let provider = create_test_provider();
        let provider_id = provider.provider_id;
        manager.register_provider(provider);

        let response = SamlResponse {
            name_id: "user@example.com".into(),
            session_index: "session123".into(),
            attributes: std::collections::HashMap::new(),
            issuer: "https://idp.example.com".into(),
            not_before: Some(Utc::now() - chrono::Duration::hours(1)),
            not_on_or_after: Some(Utc::now() + chrono::Duration::hours(1)),
        };

        let result = manager.validate_saml_response(&response, &provider_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_saml_response_expired() {
        let mut manager = SsoManager::new();
        let provider = create_test_provider();
        let provider_id = provider.provider_id;
        manager.register_provider(provider);

        let response = SamlResponse {
            name_id: "user@example.com".into(),
            session_index: "session123".into(),
            attributes: std::collections::HashMap::new(),
            issuer: "https://idp.example.com".into(),
            not_before: None,
            not_on_or_after: Some(Utc::now() - chrono::Duration::hours(1)),
        };

        let result = manager.validate_saml_response(&response, &provider_id);
        assert!(matches!(result, Err(SsoError::TokenExpired)));
    }

    #[test]
    fn test_extract_user_from_saml() {
        let mut manager = SsoManager::new();
        let provider = create_test_provider();
        let provider_id = provider.provider_id;
        manager.register_provider(provider);

        let mut attributes = std::collections::HashMap::new();
        attributes.insert("email".into(), vec!["user@example.com".into()]);
        attributes.insert("given_name".into(), vec!["John".into()]);
        attributes.insert("family_name".into(), vec!["Doe".into()]);

        let response = SamlResponse {
            name_id: "external_user_123".into(),
            session_index: "session123".into(),
            attributes,
            issuer: "https://idp.example.com".into(),
            not_before: None,
            not_on_or_after: None,
        };

        let user_info = manager.extract_user_from_saml(&response, &provider_id);
        assert!(user_info.is_ok());

        let user = user_info.unwrap();
        assert_eq!(user.email, "user@example.com");
        assert_eq!(user.first_name, Some("John".into()));
        assert_eq!(user.last_name, Some("Doe".into()));
        assert_eq!(user.external_id, "external_user_123");
    }
}
