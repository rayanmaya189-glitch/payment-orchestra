//! API Gateway TDD tests

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

fn setup() -> GatewayPipeline { GatewayPipeline::new() }

fn create_request(method: &str, path: &str, api_key: Option<&str>, body: &[u8]) -> ProcessInboundRequest {
    let mut headers = vec![("X-Request-ID".into(), Uuid::now_v7().to_string())];
    if let Some(key) = api_key {
        headers.push(("Authorization".into(), format!("Bearer {}", key)));
    }
    ProcessInboundRequest {
        http_method: method.into(),
        path: path.into(),
        headers,
        body: body.to_vec(),
        source_ip: "192.168.1.1".into(),
    }
}

#[tokio::test]
async fn test_valid_payment_intent_request() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("POST", "/v1/payment-intents", Some("sk_test_12345"), b"test_body")
    ).await.unwrap();

    assert!(result.allowed);
    assert_eq!(result.http_status, 200);
    let body_str = String::from_utf8(result.response_body).unwrap();
    assert!(body_str.contains("orchestration.v1.OrchestrationService::CreatePaymentIntent"));
}

#[tokio::test]
async fn test_route_not_found_returns_error() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("POST", "/v1/unknown", Some("sk_test_12345"), b"")
    ).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_method_not_allowed() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("GET", "/v1/payment-intents", Some("sk_test_12345"), b"")
    ).await.unwrap();

    assert!(!result.allowed);
    assert_eq!(result.http_status, 405);
}

#[tokio::test]
async fn test_auth_required_for_protected_routes() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("POST", "/v1/payment-intents", None, b"test_body")
    ).await.unwrap();

    assert!(!result.allowed);
    assert_eq!(result.http_status, 401);
}

#[tokio::test]
async fn test_invalid_api_key_rejected() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("POST", "/v1/payment-intents", Some("sk_live_bad_key"), b"test_body")
    ).await.unwrap();

    assert!(!result.allowed);
    assert_eq!(result.http_status, 401);
}

#[tokio::test]
async fn test_auth_endpoint_does_not_require_auth() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("POST", "/v1/authenticate", None, b"login_body")
    ).await.unwrap();

    assert!(result.allowed);
    assert_eq!(result.http_status, 200);
}

#[tokio::test]
async fn test_body_size_limit_enforced() {
    let pipeline = setup();
    let oversized_body = vec![0u8; MAX_BODY_SIZE + 1];
    let result = pipeline.api.process_request(
        create_request("POST", "/v1/payment-intents", Some("sk_test_12345"), &oversized_body)
    ).await.unwrap();

    assert!(!result.allowed);
    assert_eq!(result.http_status, 413);
}

#[tokio::test]
async fn test_list_routes() {
    let pipeline = setup();
    let routes = pipeline.api.list_routes().await.unwrap();
    assert!(routes.len() >= 20); // 20+ default routes
    assert!(routes.iter().any(|r| r.url_pattern == "/v1/payment-intents"));
}

#[tokio::test]
async fn test_get_specific_route() {
    let pipeline = setup();
    let route = pipeline.api.get_route("POST", "/v1/payment-intents/*/capture").await.unwrap();
    assert_eq!(route.grpc_method, "CapturePaymentIntent");
}

#[tokio::test]
async fn test_request_logging() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("POST", "/v1/invoices", Some("sk_test_12345"), b"{}")
    ).await.unwrap();

    let log = pipeline.api.get_request(result.request_id).await.unwrap();
    assert_eq!(log.http_status, 200);
    assert!(log.allowed);
}

#[tokio::test]
async fn test_health_check() {
    let pipeline = setup();
    let health = pipeline.api.health_check().await;
    assert!(health.is_healthy);
    assert!(health.routes_count >= 20);
}

#[tokio::test]
async fn test_delete_method() {
    let pipeline = setup();
    let result = pipeline.api.process_request(
        create_request("DELETE", "/v1/payment-intents/*", Some("sk_test_12345"), b"{}")
    ).await.unwrap();

    assert!(result.allowed);
    assert_eq!(result.http_status, 200);
}
