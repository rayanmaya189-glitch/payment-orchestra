//! OpenAPI spec generation for REST APIs.
//!
//! Generates OpenAPI 3.0 JSON specs for each service's REST endpoints.

use serde::{Deserialize, Serialize};

/// OpenAPI 3.0 specification root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    pub servers: Vec<Server>,
    pub paths: Vec<(String, PathItem)>,
    pub components: Option<Components>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    pub get: Option<Operation>,
    pub post: Option<Operation>,
    pub put: Option<Operation>,
    pub delete: Option<Operation>,
}

impl Default for PathItem {
    fn default() -> Self {
        Self {
            get: None,
            post: None,
            put: None,
            delete: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub summary: String,
    pub operation_id: String,
    pub tags: Vec<String>,
    pub responses: Vec<(String, Response)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub schemas: Vec<(String, Schema)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Option<Vec<(String, SchemaProperty)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProperty {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub description: Option<String>,
}

impl OpenApiSpec {
    /// Generate a basic OpenAPI spec for the operator-service.
    pub fn operator_service() -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: "Operator Service API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Operator management API for the Payment Orchestration Platform".to_string()),
            },
            servers: vec![Server {
                url: "http://localhost:8081".to_string(),
                description: Some("Local development".to_string()),
            }],
            paths: vec![
                ("/v1/operators".to_string(), PathItem {
                    post: Some(Operation {
                        summary: "Register a new operator".to_string(),
                        operation_id: "registerOperator".to_string(),
                        tags: vec!["operators".to_string()],
                        responses: vec![("201".to_string(), Response { description: "Operator created".to_string() })],
                    }),
                    get: Some(Operation {
                        summary: "List operators".to_string(),
                        operation_id: "listOperators".to_string(),
                        tags: vec!["operators".to_string()],
                        responses: vec![("200".to_string(), Response { description: "List of operators".to_string() })],
                    }),
                    ..Default::default()
                }),
            ],
            components: None,
        }
    }

    /// Generate a basic OpenAPI spec for the orchestration-service.
    pub fn orchestration_service() -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: "Orchestration Service API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Payment orchestration API".to_string()),
            },
            servers: vec![Server {
                url: "http://localhost:8085".to_string(),
                description: Some("Local development".to_string()),
            }],
            paths: vec![
                ("/v1/payment-intents".to_string(), PathItem {
                    post: Some(Operation {
                        summary: "Create a payment intent".to_string(),
                        operation_id: "createPaymentIntent".to_string(),
                        tags: vec!["payment-intents".to_string()],
                        responses: vec![("201".to_string(), Response { description: "Payment intent created".to_string() })],
                    }),
                    ..Default::default()
                }),
                ("/v1/payment-intents/{id}/authorize".to_string(), PathItem {
                    post: Some(Operation {
                        summary: "Authorize a payment intent".to_string(),
                        operation_id: "authorizePaymentIntent".to_string(),
                        tags: vec!["payment-intents".to_string()],
                        responses: vec![("200".to_string(), Response { description: "Authorization result".to_string() })],
                    }),
                    ..Default::default()
                }),
                ("/v1/payment-intents/{id}/capture".to_string(), PathItem {
                    post: Some(Operation {
                        summary: "Capture a payment".to_string(),
                        operation_id: "capturePaymentIntent".to_string(),
                        tags: vec!["payment-intents".to_string()],
                        responses: vec![("200".to_string(), Response { description: "Capture result".to_string() })],
                    }),
                    ..Default::default()
                }),
            ],
            components: None,
        }
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_service_spec() {
        let spec = OpenApiSpec::operator_service();
        assert_eq!(spec.openapi, "3.0.3");
        assert_eq!(spec.info.title, "Operator Service API");
        assert!(!spec.paths.is_empty());
    }

    #[test]
    fn test_orchestration_service_spec() {
        let spec = OpenApiSpec::orchestration_service();
        assert_eq!(spec.openapi, "3.0.3");
        assert_eq!(spec.paths.len(), 3);
    }

    #[test]
    fn test_to_json() {
        let spec = OpenApiSpec::operator_service();
        let json = spec.to_json().unwrap();
        assert!(json.contains("Operator Service API"));
        assert!(json.contains("3.0.3"));
    }
}
