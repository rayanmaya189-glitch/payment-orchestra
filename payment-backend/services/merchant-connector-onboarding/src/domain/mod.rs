//! Merchant Connector Onboarding domain model — BC-22
//!
//! BYOK (Bring Your Own Key) onboarding flow: connector selection,
//! credential schema, validation, testing, and activation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// OnboardingStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingStatus {
    /// Onboarding initiated, waiting for credential submission.
    Draft,
    /// Credentials submitted but not yet tested.
    CredentialsSubmitted,
    /// Connection test in progress.
    Testing,
    /// Connection test passed, link is active.
    Active,
    /// Link deactivated (manual).
    Deactivated,
    /// Credentials revoked / invalidated.
    Revoked,
}

impl OnboardingStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use OnboardingStatus::*;
        matches!(
            (self, target),
            (Draft, CredentialsSubmitted)
                | (CredentialsSubmitted, Testing)
                | (Testing, Active)
                | (Testing, CredentialsSubmitted) // retry
                | (Active, Deactivated)
                | (Active, Revoked)
                | (Deactivated, Active) // re-activate
                | (Deactivated, Revoked)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Revoked)
    }
}

impl std::fmt::Display for OnboardingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::CredentialsSubmitted => write!(f, "credentials_submitted"),
            Self::Testing => write!(f, "testing"),
            Self::Active => write!(f, "active"),
            Self::Deactivated => write!(f, "deactivated"),
            Self::Revoked => write!(f, "revoked"),
        }
    }
}

// ---------------------------------------------------------------------------
// CredentialField — schema for a single credential field
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub label: String,
    pub placeholder: Option<String>,
    pub validation_regex: Option<String>,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub options: Vec<FieldOption>,
    pub help_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

// ---------------------------------------------------------------------------
// ConnectorInfo — available connectors with their schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub connector_id: String,
    pub display_name: String,
    pub description: String,
    pub supported_environments: Vec<String>,
    pub supported_card_schemes: Vec<String>,
    pub supported_currencies: Vec<String>,
    pub fields: Vec<CredentialField>,
}

// ---------------------------------------------------------------------------
// HealthStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Degraded,
    Down,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Down => write!(f, "down"),
        }
    }
}

// ---------------------------------------------------------------------------
// ConnectionTestResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub merchant_name: Option<String>,
    pub permissions: Vec<String>,
}

// ---------------------------------------------------------------------------
// OnboardingRequest aggregate
// ---------------------------------------------------------------------------

/// Core OnboardingRequest aggregate root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingRequest {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub display_name: String,
    pub environment: String,
    pub status: OnboardingStatus,
    pub health_status: HealthStatus,
    pub credentials: HashMap<String, String>,
    pub encrypted_credentials: Vec<u8>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_test_result: Option<ConnectionTestResult>,
    pub credential_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OnboardingRequest {
    /// Initiate a new onboarding request in `Draft` status.
    pub fn new(
        operator_id: Uuid,
        connector_id: String,
        display_name: String,
        environment: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            link_id: Uuid::now_v7(),
            operator_id,
            connector_id,
            display_name,
            environment,
            status: OnboardingStatus::Draft,
            health_status: HealthStatus::Unknown,
            credentials: HashMap::new(),
            encrypted_credentials: Vec::new(),
            last_tested_at: None,
            last_test_result: None,
            credential_expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Submit credentials for the link.
    pub fn submit_credentials(
        &mut self,
        credentials: HashMap<String, String>,
        schema: &ConnectorInfo,
    ) -> Result<(), OnboardingError> {
        // Validate required fields
        for field in &schema.fields {
            if field.required && !credentials.contains_key(&field.name) {
                return Err(OnboardingError::MissingRequiredField(field.name.clone()));
            }
            // Validate regex if present
            if let Some(ref _regex) = field.validation_regex {
                if let Some(value) = credentials.get(&field.name) {
                    if value.is_empty() && field.required {
                        return Err(OnboardingError::InvalidFieldValue(field.name.clone()));
                    }
                    if !value.is_empty() {
                        // For Phase 1, use length-based validation only
                        if value.len() < (field.min_length.unwrap_or(0) as usize) {
                            return Err(OnboardingError::InvalidFieldValue(field.name.clone()));
                        }
                    }
                }
            }
        }

        // Store credentials (in production, would encrypt via KMS)
        self.credentials = credentials;
        self.status = OnboardingStatus::CredentialsSubmitted;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Start connection testing.
    pub fn start_test(&mut self) -> Result<(), OnboardingError> {
        if self.status != OnboardingStatus::CredentialsSubmitted {
            return Err(OnboardingError::InvalidTransition);
        }
        self.status = OnboardingStatus::Testing;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Record a successful connection test.
    pub fn record_test_success(&mut self, result: ConnectionTestResult) -> Result<(), OnboardingError> {
        if self.status != OnboardingStatus::Testing {
            return Err(OnboardingError::InvalidTransition);
        }
        self.status = OnboardingStatus::Active;
        self.health_status = HealthStatus::Healthy;
        self.last_tested_at = Some(Utc::now());
        self.last_test_result = Some(result);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Record a failed connection test.
    pub fn record_test_failure(&mut self, result: ConnectionTestResult) -> Result<(), OnboardingError> {
        if self.status != OnboardingStatus::Testing {
            return Err(OnboardingError::InvalidTransition);
        }
        // Stay in testing status for retry
        self.health_status = HealthStatus::Degraded;
        self.last_tested_at = Some(Utc::now());
        self.last_test_result = Some(result);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Deactivate the link.
    pub fn deactivate(&mut self) -> Result<(), OnboardingError> {
        if !self.status.can_transition_to(&OnboardingStatus::Deactivated) {
            return Err(OnboardingError::InvalidTransition);
        }
        self.status = OnboardingStatus::Deactivated;
        self.health_status = HealthStatus::Down;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Revoke the link credentials.
    pub fn revoke(&mut self) -> Result<(), OnboardingError> {
        if !self.status.can_transition_to(&OnboardingStatus::Revoked) {
            return Err(OnboardingError::InvalidTransition);
        }
        self.status = OnboardingStatus::Revoked;
        self.health_status = HealthStatus::Down;
        self.updated_at = Utc::now();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Default connectors registry
// ---------------------------------------------------------------------------

pub fn default_connectors() -> Vec<ConnectorInfo> {
    vec![
        ConnectorInfo {
            connector_id: "network_international".into(),
            display_name: "Network International".into(),
            description: "UAE's leading acquirer — best for local card processing".into(),
            supported_environments: vec!["sandbox".into(), "production".into()],
            supported_card_schemes: vec!["visa".into(), "mastercard".into()],
            supported_currencies: vec!["AED".into()],
            fields: vec![
                CredentialField {
                    name: "merchant_id".into(),
                    field_type: "text".into(),
                    required: true,
                    label: "Merchant ID".into(),
                    placeholder: Some("MERCHANT12345".into()),
                    validation_regex: Some("^[A-Z0-9]{8,20}$".into()),
                    min_length: Some(8),
                    max_length: Some(20),
                    options: vec![],
                    help_text: Some("Your Network International merchant ID".into()),
                },
                CredentialField {
                    name: "api_key".into(),
                    field_type: "password".into(),
                    required: true,
                    label: "API Key".into(),
                    placeholder: Some("ni_live_...".into()),
                    validation_regex: Some("^ni_(live|test)_[a-zA-Z0-9]+$".into()),
                    min_length: Some(16),
                    max_length: Some(64),
                    options: vec![],
                    help_text: Some("Find this in your NI dashboard under API Keys".into()),
                },
                CredentialField {
                    name: "environment".into(),
                    field_type: "select".into(),
                    required: true,
                    label: "Environment".into(),
                    placeholder: None,
                    validation_regex: None,
                    min_length: None,
                    max_length: None,
                    options: vec![
                        FieldOption { value: "sandbox".into(), label: "Sandbox (Testing)".into() },
                        FieldOption { value: "production".into(), label: "Production (Live)".into() },
                    ],
                    help_text: Some("Use Sandbox for testing".into()),
                },
            ],
        },
        ConnectorInfo {
            connector_id: "checkout_com".into(),
            display_name: "Checkout.com".into(),
            description: "Global PSP — multi-currency, strong fraud tools".into(),
            supported_environments: vec!["sandbox".into(), "production".into()],
            supported_card_schemes: vec!["visa".into(), "mastercard".into(), "amex".into()],
            supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into()],
            fields: vec![
                CredentialField {
                    name: "secret_key".into(),
                    field_type: "password".into(),
                    required: true,
                    label: "Secret Key".into(),
                    placeholder: Some("sk_live_...".into()),
                    validation_regex: Some("^sk_(test|live)_[a-zA-Z0-9]+$".into()),
                    min_length: Some(32),
                    max_length: Some(128),
                    options: vec![],
                    help_text: Some("Find this in your Checkout.com dashboard under Settings > API Keys".into()),
                },
                CredentialField {
                    name: "public_key".into(),
                    field_type: "password".into(),
                    required: true,
                    label: "Public Key".into(),
                    placeholder: Some("pk_live_...".into()),
                    validation_regex: Some("^pk_(test|live)_[a-zA-Z0-9]+$".into()),
                    min_length: Some(24),
                    max_length: Some(64),
                    options: vec![],
                    help_text: Some("Your public key from the same API Keys section".into()),
                },
                CredentialField {
                    name: "environment".into(),
                    field_type: "select".into(),
                    required: true,
                    label: "Environment".into(),
                    placeholder: None,
                    validation_regex: None,
                    min_length: None,
                    max_length: None,
                    options: vec![
                        FieldOption { value: "sandbox".into(), label: "Sandbox (Testing)".into() },
                        FieldOption { value: "production".into(), label: "Production (Live)".into() },
                    ],
                    help_text: Some("Use Sandbox for testing. Switch to Production when ready".into()),
                },
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum OnboardingError {
    #[error("Onboarding request not found: {0}")]
    NotFound(Uuid),
    #[error("Invalid status transition")]
    InvalidTransition,
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),
    #[error("Invalid field value: {0}")]
    InvalidFieldValue(String),
    #[error("Connector not found: {0}")]
    ConnectorNotFound(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Duplicate credentials: already in use")]
    DuplicateCredentials,
}
