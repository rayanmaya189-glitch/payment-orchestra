//! Onboarding schema types for connector-gateway.
//! Each connector declares its configuration fields for dynamic form rendering.

use serde::{Deserialize, Serialize};

/// Dynamic schema describing what credentials a connector requires.
/// The dashboard renders a form based on this schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingSchema {
    pub connector_id: String,
    pub fields: Vec<OnboardingField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub label: String,
    pub validation_regex: Option<String>,
    pub help_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Password,
    Url,
    Integer,
    Select { options: Vec<SelectOption> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}
