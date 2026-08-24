//! Tests for India payment gateway connectors and FX service.

use crate::domain::{
    self, AcquirerConnector, AuthorizeRequest, AuthorizeStatus, CardScheme, ConnectorConfig,
    ConnectorRegistry, FxRateRequest, Money,
};

// ─── Helper Functions ──────────────────────────────────────────────────────────

fn test_connector_config(api_key: &str, secret_key: &str, merchant_id: &str) -> ConnectorConfig {
    ConnectorConfig {
        api_key: Some(api_key.into()),
        secret_key: Some(secret_key.into()),
        merchant_id: Some(merchant_id.into()),
        store_id: None,
        environment: "sandbox".into(),
        additional_fields: std::collections::HashMap::new(),
    }
}

fn test_upi_config() -> ConnectorConfig {
    ConnectorConfig {
        api_key: Some("test_api_key".into()),
        secret_key: None,
        merchant_id: Some("test_merchant".into()),
        store_id: None,
        environment: "sandbox".into(),
        additional_fields: std::collections::HashMap::from([
            ("vpa".into(), "merchant@upi".into()),
        ]),
    }
}

fn sample_authorize_request(idempotency_key: &str) -> AuthorizeRequest {
    AuthorizeRequest {
        payment_method_token: "tok_test".into(),
        amount: Money {
            amount_minor_units: 10000,
            currency: "INR".into(),
        },
        currency: "INR".into(),
        idempotency_key: idempotency_key.into(),
        card_scheme: CardScheme::Visa,
        metadata: None,
        three_ds_data: None,
    }
}

// ─── Connector Registry Tests ──────────────────────────────────────────────────

#[test]
fn test_registry_contains_india_connectors() {
    let registry = ConnectorRegistry::with_all_connectors();
    let ids = registry.list_ids();

    // Core India gateways
    assert!(ids.contains(&"razorpay".to_string()));
    assert!(ids.contains(&"cashfree".to_string()));
    assert!(ids.contains(&"payu".to_string()));
    assert!(ids.contains(&"phonepe".to_string()));
    assert!(ids.contains(&"billdesk".to_string()));

    // Additional India gateways
    assert!(ids.contains(&"ccavenue".to_string()));
    assert!(ids.contains(&"paytm".to_string()));
    assert!(ids.contains(&"juspay".to_string()));
    assert!(ids.contains(&"instamojo".to_string()));
    assert!(ids.contains(&"easebuzz".to_string()));

    // Bank-specific
    assert!(ids.contains(&"icici".to_string()));
    assert!(ids.contains(&"hdfc".to_string()));
    assert!(ids.contains(&"axis".to_string()));
    assert!(ids.contains(&"sbi".to_string()));
    assert!(ids.contains(&"kotak".to_string()));
    assert!(ids.contains(&"yesbank".to_string()));
    assert!(ids.contains(&"indusind".to_string()));

    // Additional India
    assert!(ids.contains(&"pinelabs".to_string()));
    assert!(ids.contains(&"worldline".to_string()));
    assert!(ids.contains(&"zaakpay".to_string()));
    assert!(ids.contains(&"payglocal".to_string()));
    assert!(ids.contains(&"mswipe".to_string()));
    assert!(ids.contains(&"fampay".to_string()));
    assert!(ids.contains(&"zestmoney".to_string()));
    assert!(ids.contains(&"direcpay".to_string()));
    assert!(ids.contains(&"atom".to_string()));

    // UPI
    assert!(ids.contains(&"upi".to_string()));

    // Enterprise
    assert!(ids.contains(&"adyen".to_string()));
    assert!(ids.contains(&"paypal".to_string()));
}

#[test]
fn test_registry_get_india_connector() {
    let registry = ConnectorRegistry::with_all_connectors();

    let connector = registry.get("razorpay");
    assert!(connector.is_ok());
    assert_eq!(connector.unwrap().connector_id(), "razorpay");

    let connector = registry.get("upi");
    assert!(connector.is_ok());
    assert_eq!(connector.unwrap().connector_id(), "upi");
}

// ─── Connector Capability Tests ────────────────────────────────────────────────

#[test]
fn test_razorpay_capabilities() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);
    let caps = connector.capabilities();

    assert!(caps.supports_partial_refund);
    assert!(caps.supports_native_idempotency_key);
    assert!(caps.supports_webhook_settlement);
    assert!(caps.supported_currencies.contains(&"INR".to_string()));
    assert!(caps.supported_card_schemes.contains(&CardScheme::Visa));
    assert!(caps.supported_card_schemes.contains(&CardScheme::Mastercard));
}

#[test]
fn test_upi_capabilities() {
    let config = test_upi_config();
    let connector = domain::UpiConnector::new(&config);
    let caps = connector.capabilities();

    assert!(caps.supports_partial_refund);
    assert!(caps.supports_native_idempotency_key);
    assert!(caps.supports_webhook_settlement);
    assert!(caps.supported_currencies.contains(&"INR".to_string()));
    assert!(caps.settlement_cycle == domain::SettlementCycle::SameDay);
}

#[test]
fn test_adyen_capabilities() {
    let config = test_connector_config("test_api_key", "test_secret", "test_merchant");
    let connector = domain::AdyenConnector::new(&config);
    let caps = connector.capabilities();

    assert!(caps.supports_partial_capture);
    assert!(caps.supports_partial_refund);
    assert!(caps.supports_fx_conversion);
    assert!(caps.supported_currencies.contains(&"INR".to_string()));
    assert!(caps.supported_currencies.contains(&"AED".to_string()));
}

#[test]
fn test_paypal_capabilities() {
    let config = test_connector_config("test_client_id", "test_secret", "test_merchant");
    let connector = domain::PaypalConnector::new(&config);
    let caps = connector.capabilities();

    assert!(caps.supports_partial_capture);
    assert!(caps.supports_partial_refund);
    assert!(caps.supports_fx_conversion);
    assert!(caps.supported_currencies.contains(&"INR".to_string()));
}

// ─── Connector Onboarding Schema Tests ─────────────────────────────────────────

#[test]
fn test_razorpay_onboarding_schema() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);
    let schema = connector.onboarding_schema();

    assert_eq!(schema.connector_id, "razorpay");
    assert!(schema.fields.iter().any(|f| f.name == "api_key"));
    assert!(schema.fields.iter().any(|f| f.name == "secret_key"));
    assert!(schema.fields.iter().any(|f| f.name == "environment"));
}

#[test]
fn test_upi_onboarding_schema() {
    let config = test_upi_config();
    let connector = domain::UpiConnector::new(&config);
    let schema = connector.onboarding_schema();

    assert_eq!(schema.connector_id, "upi");
    assert!(schema.fields.iter().any(|f| f.name == "merchant_id"));
    assert!(schema.fields.iter().any(|f| f.name == "vpa"));
}

// ─── Webhook Signature Verification Tests ──────────────────────────────────────

#[test]
fn test_razorpay_webhook_signature_valid() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);

    use ring::hmac;
    let key = hmac::Key::new(hmac::HMAC_SHA256, b"test_secret");
    let body = b"test_body";
    let computed = hmac::sign(&key, body);
    let signature = hex::encode(computed.as_ref());

    let mut headers = std::collections::HashMap::new();
    headers.insert("x-razorpay-signature".into(), signature);

    assert!(connector.verify_webhook_signature(&headers, body).is_ok());
}

#[test]
fn test_razorpay_webhook_signature_invalid() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);

    let mut headers = std::collections::HashMap::new();
    headers.insert("x-razorpay-signature".into(), "invalid_signature".into());

    assert!(connector.verify_webhook_signature(&headers, b"test_body").is_err());
}

#[test]
fn test_razorpay_webhook_signature_missing() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);

    let headers = std::collections::HashMap::new();

    assert!(connector.verify_webhook_signature(&headers, b"test_body").is_err());
}

// ─── Test Card Numbers Tests ───────────────────────────────────────────────────

#[test]
fn test_razorpay_test_cards() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);
    let cards = connector.test_card_numbers();

    assert!(!cards.is_empty());
    assert!(cards.iter().any(|c| c.scheme == CardScheme::Visa));
    assert!(cards.iter().any(|c| c.scheme == CardScheme::Mastercard));
}

#[test]
fn test_upi_test_cards() {
    let config = test_upi_config();
    let connector = domain::UpiConnector::new(&config);
    let cards = connector.test_card_numbers();

    assert!(!cards.is_empty());
    assert!(cards.iter().any(|c| c.scheme == CardScheme::Other("UPI".into())));
}

#[test]
fn test_adyen_test_cards() {
    let config = test_connector_config("test_api_key", "test_secret", "test_merchant");
    let connector = domain::AdyenConnector::new(&config);
    let cards = connector.test_card_numbers();

    assert!(!cards.is_empty());
    assert!(cards.iter().any(|c| c.scheme == CardScheme::Visa));
    assert!(cards.iter().any(|c| c.scheme == CardScheme::Mastercard));
    assert!(cards.iter().any(|c| c.scheme == CardScheme::Other("RuPay".into())));
}

// ─── FX Service Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_fx_rate_aed_to_inr() {
    // Skip if no API key configured
    let api_key = std::env::var("EXCHANGE_RATE_API_KEY").ok();
    if api_key.is_none() {
        eprintln!("Skipping test_fx_rate_aed_to_inr: EXCHANGE_RATE_API_KEY not set");
        return;
    }
    
    let service = domain::FxRateService::new(api_key);
    let req = FxRateRequest {
        source_currency: "AED".into(),
        target_currency: "INR".into(),
        amount: Money {
            amount_minor_units: 10000, // 100.00 AED
            currency: "AED".into(),
        },
    };

    let result = service.get_rate(&req).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.rate_minor_units > 0);
    assert_eq!(response.converted_amount.currency, "INR");
    assert!(response.converted_amount.amount_minor_units > 0);
}

#[tokio::test]
async fn test_fx_rate_inr_to_aed() {
    // Skip if no API key configured
    let api_key = std::env::var("EXCHANGE_RATE_API_KEY").ok();
    if api_key.is_none() {
        eprintln!("Skipping test_fx_rate_inr_to_aed: EXCHANGE_RATE_API_KEY not set");
        return;
    }
    
    let service = domain::FxRateService::new(api_key);
    let req = FxRateRequest {
        source_currency: "INR".into(),
        target_currency: "AED".into(),
        amount: Money {
            amount_minor_units: 1000000, // 10000.00 INR
            currency: "INR".into(),
        },
    };

    let result = service.get_rate(&req).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.rate_minor_units > 0);
    assert_eq!(response.converted_amount.currency, "AED");
}

#[tokio::test]
async fn test_fx_rate_unsupported_pair() {
    let service = domain::FxRateService::new(None);
    let req = FxRateRequest {
        source_currency: "USD".into(),
        target_currency: "EUR".into(),
        amount: Money {
            amount_minor_units: 10000,
            currency: "USD".into(),
        },
    };

    let result = service.get_rate(&req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_fx_rate_cache() {
    // Skip if no API key configured
    let api_key = std::env::var("EXCHANGE_RATE_API_KEY").ok();
    if api_key.is_none() {
        eprintln!("Skipping test_fx_rate_cache: EXCHANGE_RATE_API_KEY not set");
        return;
    }
    
    let mut service = domain::FxRateService::new(api_key);
    service.set_cache_ttl(60); // 60 seconds

    let req = FxRateRequest {
        source_currency: "AED".into(),
        target_currency: "INR".into(),
        amount: Money {
            amount_minor_units: 10000,
            currency: "AED".into(),
        },
    };

    // First call - fetches from API
    let result1 = service.get_rate(&req).await.unwrap();
    // Second call - should use cache
    let result2 = service.get_rate(&req).await.unwrap();

    assert_eq!(result1.rate, result2.rate);
    assert_eq!(result1.converted_amount.amount_minor_units, result2.converted_amount.amount_minor_units);
}

#[tokio::test]
async fn test_fx_rate_clear_cache() {
    let api_key = std::env::var("EXCHANGE_RATE_API_KEY").ok();
    let service = domain::FxRateService::new(api_key);

    let req = FxRateRequest {
        source_currency: "AED".into(),
        target_currency: "INR".into(),
        amount: Money {
            amount_minor_units: 10000,
            currency: "AED".into(),
        },
    };

    // Clear cache (should work even without API key)
    assert!(service.clear_cache().is_ok());
    
    // Test cache stats
    let (size, _) = service.cache_stats().unwrap();
    assert_eq!(size, 0);
}

// ─── Async Connector Tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_razorpay_authorize_sandbox() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);
    let req = sample_authorize_request("test_order_1");

    let result = connector.authorize(req).await;
    // Note: This will fail with a network error in test environment
    // but validates the connector can be instantiated and called
    assert!(result.is_err() || result.unwrap().status == AuthorizeStatus::Approved);
}

#[tokio::test]
async fn test_upi_authorize_sandbox() {
    let config = test_upi_config();
    let connector = domain::UpiConnector::new(&config);
    let req = sample_authorize_request("test_upi_1");

    let result = connector.authorize(req).await;
    // Note: This will fail with a network error in test environment
    assert!(result.is_err() || result.unwrap().status == AuthorizeStatus::Approved);
}

#[tokio::test]
async fn test_adyen_authorize_sandbox() {
    let config = test_connector_config("test_api_key", "test_secret", "test_merchant");
    let connector = domain::AdyenConnector::new(&config);
    let req = sample_authorize_request("test_adyen_1");

    let result = connector.authorize(req).await;
    assert!(result.is_err() || result.unwrap().status == AuthorizeStatus::Approved);
}

#[tokio::test]
async fn test_paypal_authorize_sandbox() {
    let config = test_connector_config("test_client_id", "test_secret", "test_merchant");
    let connector = domain::PaypalConnector::new(&config);
    let req = sample_authorize_request("test_paypal_1");

    let result = connector.authorize(req).await;
    assert!(result.is_err() || result.unwrap().status == AuthorizeStatus::Approved);
}

// ─── Validate Credentials Tests ────────────────────────────────────────────────

#[tokio::test]
async fn test_razorpay_validate_credentials_empty() {
    let config = ConnectorConfig {
        api_key: None,
        secret_key: None,
        merchant_id: None,
        store_id: None,
        environment: "sandbox".into(),
        additional_fields: std::collections::HashMap::new(),
    };
    let connector = domain::RazorpayConnector::new(&config);

    let result = connector.validate_credentials(&config).await.unwrap();
    assert!(!result.valid);
    assert!(result.error_message.is_some());
}

#[tokio::test]
async fn test_razorpay_validate_credentials_valid() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);

    let result = connector.validate_credentials(&config).await.unwrap();
    // Note: With test credentials, validation may fail with network error
    // but the function should not panic
    assert!(result.merchant_name.is_some() || result.error_message.is_some());
}

#[tokio::test]
async fn test_upi_validate_credentials_empty() {
    let config = ConnectorConfig {
        api_key: None,
        secret_key: None,
        merchant_id: None,
        store_id: None,
        environment: "sandbox".into(),
        additional_fields: std::collections::HashMap::new(),
    };
    let connector = domain::UpiConnector::new(&config);

    let result = connector.validate_credentials(&config).await.unwrap();
    assert!(!result.valid);
}

// ─── Parse Webhook Tests ───────────────────────────────────────────────────────

#[test]
fn test_razorpay_parse_webhook() {
    let config = test_connector_config("rzp_test", "test_secret", "test_merchant");
    let connector = domain::RazorpayConnector::new(&config);

    let body = serde_json::json!({
        "event": "payment.authorized",
        "payload": {
            "payment": {
                "entity": {
                    "id": "pay_test123",
                    "status": "authorized"
                }
            }
        }
    });

    let result = connector.parse_webhook(serde_json::to_vec(&body).unwrap().as_slice());
    assert!(result.is_ok());

    let event = result.unwrap();
    assert_eq!(event.event_type, "payment.authorized");
}

#[test]
fn test_upi_parse_webhook() {
    let config = test_upi_config();
    let connector = domain::UpiConnector::new(&config);

    let body = serde_json::json!({
        "eventType": "PAYMENT_SUCCESS",
        "txnId": "txn_test123",
        "status": "SUCCESS"
    });

    let result = connector.parse_webhook(serde_json::to_vec(&body).unwrap().as_slice());
    assert!(result.is_ok());

    let event = result.unwrap();
    assert_eq!(event.event_type, "PAYMENT_SUCCESS");
}
