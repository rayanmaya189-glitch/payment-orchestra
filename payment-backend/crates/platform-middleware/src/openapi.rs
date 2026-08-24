/// OpenAPI specification generation for the Payment Orchestra API.
///
/// This module generates a comprehensive OpenAPI 3.0 specification
/// from the protobuf service definitions and REST route mappings.

use serde::{Deserialize, Serialize};

/// OpenAPI 3.0 specification root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: ApiInfo,
    pub servers: Vec<Server>,
    pub paths: Paths,
    pub components: Components,
    pub security: Vec<SecurityRequirement>,
}

/// API information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub title: String,
    pub description: String,
    pub version: String,
    pub contact: Option<ContactInfo>,
    pub license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub name: Option<String>,
    pub url: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    pub url: Option<String>,
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    pub description: String,
}

/// API paths.
pub type Paths = std::collections::HashMap<String, PathItem>;

/// Path item containing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
}

/// HTTP operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub operation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    pub responses: Responses,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<SecurityRequirement>>,
    #[serde(rename = "tags")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String, // "path", "query", "header"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
    #[serde(rename = "type")]
    pub param_type: String,
}

/// Request body definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
    pub content: std::collections::HashMap<String, MediaType>,
}

/// Media type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    #[serde(rename = "schema")]
    pub schema: SchemaRef,
}

/// Schema reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaRef {
    Reference { #[serde(rename = "$ref")] reference: String },
    Schema(Schema),
}

/// Schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::HashMap<String, SchemaRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Response definitions.
pub type Responses = std::collections::HashMap<String, Response>;

/// Response definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<std::collections::HashMap<String, MediaType>>,
}

/// Components definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub security_schemes: std::collections::HashMap<String, SecurityScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<std::collections::HashMap<String, Schema>>,
}

/// Security scheme definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SecurityScheme {
    #[serde(rename = "apiKey")]
    ApiKey {
        #[serde(rename = "in")]
        location: String,
        name: String,
        description: Option<String>,
    },
    #[serde(rename = "http")]
    Http {
        scheme: String,
        bearer_format: Option<String>,
        description: Option<String>,
    },
}

/// Security requirement.
pub type SecurityRequirement = std::collections::HashMap<String, Vec<String>>;

/// Generate the complete OpenAPI specification for Payment Orchestra.
pub fn generate_openapi_spec() -> String {
    let spec = OpenApiSpec {
        openapi: "3.0.3".to_string(),
        info: ApiInfo {
            title: "Payment Orchestra API".to_string(),
            description: "Payment Orchestration Platform - Route, optimize, and manage payments across multiple providers.".to_string(),
            version: "1.0.0".to_string(),
            contact: Some(ContactInfo {
                name: Some("Payment Orchestra Support".to_string()),
                url: Some("https://docs.paymentorchestra.com".to_string()),
                email: Some("support@paymentorchestra.com".to_string()),
            }),
            license: Some(License {
                name: "MIT".to_string(),
                url: Some("https://opensource.org/licenses/MIT".to_string()),
            }),
        },
        servers: vec![
            Server {
                url: "https://sandbox.api.paymentorchestra.com".to_string(),
                description: "Sandbox Environment".to_string(),
            },
            Server {
                url: "https://api.paymentorchestra.com".to_string(),
                description: "Production Environment".to_string(),
            },
        ],
        paths: generate_paths(),
        components: generate_components(),
        security: vec![
            std::collections::HashMap::from([
                ("BearerAuth".to_string(), vec![]),
            ]),
        ],
    };

    serde_json::to_string_pretty(&spec).unwrap_or_else(|_| "{}".to_string())
}

/// Generate API paths.
fn generate_paths() -> Paths {
    let mut paths = std::collections::HashMap::new();

    // Payment Intents
    paths.insert("/v1/payment-intents".to_string(), PathItem {
        get: Some(Operation {
            summary: "List Payment Intents".to_string(),
            description: Some("Retrieve a list of payment intents with optional filtering.".to_string()),
            operation_id: "listPaymentIntents".to_string(),
            parameters: Some(vec![
                Parameter { name: "limit".to_string(), location: "query".to_string(), description: Some("Max results (1-100)".to_string()), required: false, param_type: "integer".to_string() },
                Parameter { name: "offset".to_string(), location: "query".to_string(), description: Some("Results offset".to_string()), required: false, param_type: "integer".to_string() },
                Parameter { name: "status".to_string(), location: "query".to_string(), description: Some("Filter by status".to_string()), required: false, param_type: "string".to_string() },
            ]),
            request_body: None,
            responses: generate_list_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        post: Some(Operation {
            summary: "Create Payment Intent".to_string(),
            description: Some("Create a new payment intent for processing a payment.".to_string()),
            operation_id: "createPaymentIntent".to_string(),
            parameters: None,
            request_body: Some(RequestBody {
                description: Some("Payment intent creation parameters".to_string()),
                required: true,
                content: generate_json_content("CreatePaymentIntentRequest"),
            }),
            responses: generate_single_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        put: None,
        patch: None,
        delete: None,
    });

    paths.insert("/v1/payment-intents/{id}".to_string(), PathItem {
        get: Some(Operation {
            summary: "Get Payment Intent".to_string(),
            description: Some("Retrieve a payment intent by ID.".to_string()),
            operation_id: "getPaymentIntent".to_string(),
            parameters: Some(vec![
                Parameter { name: "id".to_string(), location: "path".to_string(), description: Some("Payment intent ID".to_string()), required: true, param_type: "string".to_string() },
            ]),
            request_body: None,
            responses: generate_single_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        post: None,
        put: None,
        patch: Some(Operation {
            summary: "Update Payment Intent".to_string(),
            description: Some("Update a payment intent's metadata.".to_string()),
            operation_id: "updatePaymentIntent".to_string(),
            parameters: Some(vec![
                Parameter { name: "id".to_string(), location: "path".to_string(), description: Some("Payment intent ID".to_string()), required: true, param_type: "string".to_string() },
            ]),
            request_body: Some(RequestBody {
                description: Some("Payment intent update parameters".to_string()),
                required: true,
                content: generate_json_content("UpdatePaymentIntentRequest"),
            }),
            responses: generate_single_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        delete: None,
    });

    // Capture, Void, Refund endpoints
    paths.insert("/v1/payment-intents/{id}/authorize".to_string(), PathItem {
        get: None,
        post: Some(Operation {
            summary: "Authorize Payment Intent".to_string(),
            description: Some("Authorize a payment intent for processing.".to_string()),
            operation_id: "authorizePaymentIntent".to_string(),
            parameters: Some(vec![
                Parameter { name: "id".to_string(), location: "path".to_string(), description: Some("Payment intent ID".to_string()), required: true, param_type: "string".to_string() },
            ]),
            request_body: None,
            responses: generate_single_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        put: None,
        patch: None,
        delete: None,
    });

    paths.insert("/v1/payment-intents/{id}/capture".to_string(), PathItem {
        get: None,
        post: Some(Operation {
            summary: "Capture Payment Intent".to_string(),
            description: Some("Capture a previously authorized payment intent.".to_string()),
            operation_id: "capturePaymentIntent".to_string(),
            parameters: Some(vec![
                Parameter { name: "id".to_string(), location: "path".to_string(), description: Some("Payment intent ID".to_string()), required: true, param_type: "string".to_string() },
            ]),
            request_body: Some(RequestBody {
                description: Some("Capture parameters (optional amount for partial capture)".to_string()),
                required: false,
                content: generate_json_content("CapturePaymentIntentRequest"),
            }),
            responses: generate_single_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        put: None,
        patch: None,
        delete: None,
    });

    paths.insert("/v1/payment-intents/{id}/void".to_string(), PathItem {
        get: None,
        post: Some(Operation {
            summary: "Void Payment Intent".to_string(),
            description: Some("Void (cancel) a payment intent before capture.".to_string()),
            operation_id: "voidPaymentIntent".to_string(),
            parameters: Some(vec![
                Parameter { name: "id".to_string(), location: "path".to_string(), description: Some("Payment intent ID".to_string()), required: true, param_type: "string".to_string() },
            ]),
            request_body: None,
            responses: generate_single_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        put: None,
        patch: None,
        delete: None,
    });

    paths.insert("/v1/payment-intents/{id}/refund".to_string(), PathItem {
        get: None,
        post: Some(Operation {
            summary: "Refund Payment Intent".to_string(),
            description: Some("Refund a captured payment intent.".to_string()),
            operation_id: "refundPaymentIntent".to_string(),
            parameters: Some(vec![
                Parameter { name: "id".to_string(), location: "path".to_string(), description: Some("Payment intent ID".to_string()), required: true, param_type: "string".to_string() },
            ]),
            request_body: Some(RequestBody {
                description: Some("Refund parameters (optional amount for partial refund)".to_string()),
                required: false,
                content: generate_json_content("RefundPaymentIntentRequest"),
            }),
            responses: generate_single_response("PaymentIntent"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Payment Intents".to_string()]),
        }),
        put: None,
        patch: None,
        delete: None,
    });

    // Gateway Profiles
    paths.insert("/v1/gateway-profiles".to_string(), PathItem {
        get: Some(Operation {
            summary: "List Gateway Profiles".to_string(),
            description: Some("Retrieve all configured gateway profiles.".to_string()),
            operation_id: "listGatewayProfiles".to_string(),
            parameters: None,
            request_body: None,
            responses: generate_list_response("GatewayProfile"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Gateway Profiles".to_string()]),
        }),
        post: Some(Operation {
            summary: "Create Gateway Profile".to_string(),
            description: Some("Create a new gateway profile with connector credentials.".to_string()),
            operation_id: "createGatewayProfile".to_string(),
            parameters: None,
            request_body: Some(RequestBody {
                description: Some("Gateway profile creation parameters".to_string()),
                required: true,
                content: generate_json_content("CreateGatewayProfileRequest"),
            }),
            responses: generate_single_response("GatewayProfile"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Gateway Profiles".to_string()]),
        }),
        put: None,
        patch: None,
        delete: None,
    });

    // Routing Policies
    paths.insert("/v1/routing-policies".to_string(), PathItem {
        get: Some(Operation {
            summary: "List Routing Policies".to_string(),
            description: Some("Retrieve all routing policies.".to_string()),
            operation_id: "listRoutingPolicies".to_string(),
            parameters: None,
            request_body: None,
            responses: generate_list_response("RoutingPolicy"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Routing Policies".to_string()]),
        }),
        post: Some(Operation {
            summary: "Create Routing Policy".to_string(),
            description: Some("Create a new routing policy with rules.".to_string()),
            operation_id: "createRoutingPolicy".to_string(),
            parameters: None,
            request_body: Some(RequestBody {
                description: Some("Routing policy creation parameters".to_string()),
                required: true,
                content: generate_json_content("CreateRoutingPolicyRequest"),
            }),
            responses: generate_single_response("RoutingPolicy"),
            security: Some(vec![std::collections::HashMap::new()]),
            tags: Some(vec!["Routing Policies".to_string()]),
        }),
        put: None,
        patch: None,
        delete: None,
    });

    paths
}

/// Generate component schemas and security schemes.
fn generate_components() -> Components {
    let mut security_schemes = std::collections::HashMap::new();
    security_schemes.insert("BearerAuth".to_string(), SecurityScheme::Http {
        scheme: "bearer".to_string(),
        bearer_format: Some("API Key".to_string()),
        description: Some("API key authentication. Use your public key (pk_test_* or pk_live_*) in the Authorization header.".to_string()),
    });

    Components {
        security_schemes,
        schemas: None, // Schemas would be auto-generated from protobuf in production
    }
}

/// Generate JSON content for request/response.
fn generate_json_content(schema_name: &str) -> std::collections::HashMap<String, MediaType> {
    let mut content = std::collections::HashMap::new();
    content.insert("application/json".to_string(), MediaType {
        schema: SchemaRef::Reference {
            reference: format!("#/components/schemas/{}", schema_name),
        },
    });
    content
}

/// Generate single object response.
fn generate_single_response(schema_name: &str) -> Responses {
    let mut responses = std::collections::HashMap::new();
    responses.insert("200".to_string(), Response {
        description: format!("Successful response returning a {}", schema_name),
        content: Some(generate_json_content(schema_name)),
    });
    responses.insert("400".to_string(), Response {
        description: "Bad request - invalid parameters".to_string(),
        content: None,
    });
    responses.insert("401".to_string(), Response {
        description: "Unauthorized - invalid API key".to_string(),
        content: None,
    });
    responses.insert("404".to_string(), Response {
        description: "Not found".to_string(),
        content: None,
    });
    responses.insert("429".to_string(), Response {
        description: "Rate limit exceeded".to_string(),
        content: None,
    });
    responses
}

/// Generate list response.
fn generate_list_response(item_name: &str) -> Responses {
    let mut responses = std::collections::HashMap::new();
    responses.insert("200".to_string(), Response {
        description: format!("Successful response returning a list of {}s", item_name),
        content: Some(generate_json_content(&format!("{}List", item_name))),
    });
    responses.insert("400".to_string(), Response {
        description: "Bad request - invalid parameters".to_string(),
        content: None,
    });
    responses.insert("401".to_string(), Response {
        description: "Unauthorized - invalid API key".to_string(),
        content: None,
    });
    responses.insert("429".to_string(), Response {
        description: "Rate limit exceeded".to_string(),
        content: None,
    });
    responses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_openapi_spec() {
        let spec = generate_openapi_spec();
        assert!(!spec.is_empty());
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&spec).unwrap();
        assert_eq!(parsed["openapi"], "3.0.3");
        assert_eq!(parsed["info"]["title"], "Payment Orchestra API");
        assert_eq!(parsed["info"]["version"], "1.0.0");
    }
}
