//! Domain events for merchant-connector-onboarding

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnboardingEvent {
    Initiated(OnboardingInitiated),
    CredentialsSubmitted(OnboardingCredentialsSubmitted),
    TestStarted(OnboardingTestStarted),
    TestCompleted(OnboardingTestCompleted),
    Activated(OnboardingActivated),
    Deactivated(OnboardingDeactivated),
    Revoked(OnboardingRevoked),
}

pub const EVENT_TYPE_INITIATED: &str = "onboarding.initiated";
pub const EVENT_TYPE_CREDENTIALS_SUBMITTED: &str = "onboarding.credentials_submitted";
pub const EVENT_TYPE_TEST_STARTED: &str = "onboarding.test_started";
pub const EVENT_TYPE_TEST_COMPLETED: &str = "onboarding.test_completed";
pub const EVENT_TYPE_ACTIVATED: &str = "onboarding.activated";
pub const EVENT_TYPE_DEACTIVATED: &str = "onboarding.deactivated";
pub const EVENT_TYPE_REVOKED: &str = "onboarding.revoked";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingInitiated {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub environment: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingCredentialsSubmitted {
    pub link_id: Uuid,
    pub field_count: usize,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingTestStarted {
    pub link_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingTestCompleted {
    pub link_id: Uuid,
    pub success: bool,
    pub latency_ms: u64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingActivated {
    pub link_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingDeactivated {
    pub link_id: Uuid,
    pub reason: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingRevoked {
    pub link_id: Uuid,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}
