use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::error::OnboardingError;
use super::status::{HealthStatus, OnboardingStatus};
use super::types::{ConnectionTestResult, ConnectorInfo};

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

    pub fn submit_credentials(
        &mut self,
        credentials: HashMap<String, String>,
        schema: &ConnectorInfo,
    ) -> Result<(), OnboardingError> {
        for field in &schema.fields {
            if field.required && !credentials.contains_key(&field.name) {
                return Err(OnboardingError::MissingRequiredField(field.name.clone()));
            }
            if let Some(ref _regex) = field.validation_regex {
                if let Some(value) = credentials.get(&field.name) {
                    if value.is_empty() && field.required {
                        return Err(OnboardingError::InvalidFieldValue(field.name.clone()));
                    }
                    if !value.is_empty() {
                        if value.len() < (field.min_length.unwrap_or(0) as usize) {
                            return Err(OnboardingError::InvalidFieldValue(field.name.clone()));
                        }
                    }
                }
            }
        }
        self.credentials = credentials;
        self.status = OnboardingStatus::CredentialsSubmitted;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn start_test(&mut self) -> Result<(), OnboardingError> {
        if self.status != OnboardingStatus::CredentialsSubmitted {
            return Err(OnboardingError::InvalidTransition);
        }
        self.status = OnboardingStatus::Testing;
        self.updated_at = Utc::now();
        Ok(())
    }

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

    pub fn record_test_failure(&mut self, result: ConnectionTestResult) -> Result<(), OnboardingError> {
        if self.status != OnboardingStatus::Testing {
            return Err(OnboardingError::InvalidTransition);
        }
        self.health_status = HealthStatus::Degraded;
        self.last_tested_at = Some(Utc::now());
        self.last_test_result = Some(result);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<(), OnboardingError> {
        if !self.status.can_transition_to(&OnboardingStatus::Deactivated) {
            return Err(OnboardingError::InvalidTransition);
        }
        self.status = OnboardingStatus::Deactivated;
        self.health_status = HealthStatus::Down;
        self.updated_at = Utc::now();
        Ok(())
    }

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
