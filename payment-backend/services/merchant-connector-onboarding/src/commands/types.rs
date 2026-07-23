//! Command types for merchant-connector-onboarding

use std::collections::HashMap;
use uuid::Uuid;
use crate::domain::*;

pub struct InitiateOnboardingCommand {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub display_name: String,
    pub environment: String,
}

pub struct SubmitCredentialsCommand {
    pub link_id: Uuid,
    pub credentials: HashMap<String, String>,
}

pub struct TestConnectionCommand {
    pub link_id: Uuid,
}

pub struct CompleteTestCommand {
    pub link_id: Uuid,
    pub result: ConnectionTestResult,
}

pub struct DeactivateLinkCommand {
    pub link_id: Uuid,
    pub reason: Option<String>,
}

pub struct RevokeLinkCommand {
    pub link_id: Uuid,
    pub reason: String,
}
