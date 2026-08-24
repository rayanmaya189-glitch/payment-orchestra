use serde::{Deserialize, Serialize};

/// Returns the default set of supported connectors with their credential schemas.
pub fn default_connectors() -> Vec<ConnectorInfo> {
    vec![
        ConnectorInfo {
            connector_id: "network_international".into(),
            display_name: "Network International".into(),
            description: "Leading payment gateway in the Middle East and Africa".into(),
            supported_environments: vec!["sandbox".into(), "production".into()],
            supported_card_schemes: vec!["visa".into(), "mastercard".into()],
            supported_currencies: vec!["AED".into()],
            fields: vec![
                CredentialField {
                    name: "merchant_id".into(),
                    field_type: "text".into(),
                    required: true,
                    label: "Merchant ID".into(),
                    placeholder: Some("e.g., MER-12345".into()),
                    validation_regex: Some(r"^MER-\d{5,10}$".into()),
                    min_length: Some(5),
                    max_length: Some(20),
                    options: vec![],
                    help_text: Some("Your Network International merchant identifier".into()),
                },
                CredentialField {
                    name: "api_key".into(),
                    field_type: "password".into(),
                    required: true,
                    label: "API Key".into(),
                    placeholder: Some("Enter your API key".into()),
                    validation_regex: Some(r"^[A-Za-z0-9+/=]{32,64}$".into()),
                    min_length: Some(32),
                    max_length: Some(64),
                    options: vec![],
                    help_text: Some("Your Network International API key".into()),
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
                        FieldOption { value: "sandbox".into(), label: "Sandbox".into() },
                        FieldOption { value: "production".into(), label: "Production".into() },
                    ],
                    help_text: Some("Select the environment to connect to".into()),
                },
            ],
        },
        ConnectorInfo {
            connector_id: "checkout_com".into(),
            display_name: "Checkout.com".into(),
            description: "Global payment gateway with comprehensive fraud detection".into(),
            supported_environments: vec!["sandbox".into(), "production".into()],
            supported_card_schemes: vec!["visa".into(), "mastercard".into(), "amex".into()],
            supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into()],
            fields: vec![
                CredentialField {
                    name: "secret_key".into(),
                    field_type: "password".into(),
                    required: true,
                    label: "Secret Key".into(),
                    placeholder: Some("sk_test_...".into()),
                    validation_regex: Some(r"^sk_(test|live)_[a-zA-Z0-9]+$".into()),
                    min_length: Some(20),
                    max_length: Some(128),
                    options: vec![],
                    help_text: Some("Your Checkout.com secret API key".into()),
                },
                CredentialField {
                    name: "public_key".into(),
                    field_type: "password".into(),
                    required: true,
                    label: "Public Key".into(),
                    placeholder: Some("pk_test_...".into()),
                    validation_regex: Some(r"^pk_(test|live)_[a-zA-Z0-9]+$".into()),
                    min_length: Some(20),
                    max_length: Some(128),
                    options: vec![],
                    help_text: Some("Your Checkout.com public API key".into()),
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
                        FieldOption { value: "sandbox".into(), label: "Sandbox".into() },
                        FieldOption { value: "production".into(), label: "Production".into() },
                    ],
                    help_text: Some("Select the environment to connect to".into()),
                },
            ],
        },
    ]
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub merchant_name: Option<String>,
    pub permissions: Vec<String>,
}
