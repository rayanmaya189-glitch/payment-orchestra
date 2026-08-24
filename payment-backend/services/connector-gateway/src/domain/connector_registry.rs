use std::collections::HashMap;

use super::connector::AcquirerConnector;
use super::error::ConnectorError;
use super::types::ConnectorConfig;

/// Registry of all available acquirer connectors.
pub struct ConnectorRegistry {
    connectors: HashMap<String, Box<dyn AcquirerConnector>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn register(&mut self, connector: Box<dyn AcquirerConnector>) {
        let id = connector.connector_id().to_string();
        self.connectors.insert(id, connector);
    }

    pub fn get(&self, connector_id: &str) -> Result<&dyn AcquirerConnector, ConnectorError> {
        self.connectors
            .get(connector_id)
            .map(|c| c.as_ref())
            .ok_or_else(|| ConnectorError::InvalidRequest(format!("Connector '{}' not found", connector_id)))
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.connectors.keys().cloned().collect()
    }

    pub fn list_all(&self) -> Vec<&dyn AcquirerConnector> {
        self.connectors.values().map(|c| c.as_ref()).collect()
    }
}

impl ConnectorRegistry {
    /// Create a new registry pre-populated with all known connectors.
    pub fn with_all_connectors() -> Self {
        let mut registry = Self::new();

        // Register the real Stripe connector
        let stripe_config = ConnectorConfig {
            api_key: Some("sk_test_placeholder".into()),
            secret_key: Some("sk_test_placeholder".into()),
            merchant_id: None,
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::stripe_connector::StripeConnector::new(&stripe_config)));

        // Register UAE payment gateway connectors
        let checkout_config = ConnectorConfig {
            api_key: None,
            secret_key: Some("sk_test_placeholder".into()),
            merchant_id: None,
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::checkout_com_connector::CheckoutComConnector::new(&checkout_config)));

        let ni_config = ConnectorConfig {
            api_key: Some("test_api_key".into()),
            secret_key: None,
            merchant_id: Some("test_merchant".into()),
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::network_intl_connector::NetworkIntlConnector::new(&ni_config)));

        let telr_config = ConnectorConfig {
            api_key: Some("test_api_key".into()),
            secret_key: None,
            merchant_id: None,
            store_id: Some("test_store".into()),
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::telr_connector::TelrConnector::new(&telr_config)));

        let tap_config = ConnectorConfig {
            api_key: None,
            secret_key: Some("pk_test_placeholder".into()),
            merchant_id: None,
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::tap_payments_connector::TapPaymentsConnector::new(&tap_config)));

        let paytabs_config = ConnectorConfig {
            api_key: None,
            secret_key: Some("test_server_key".into()),
            merchant_id: Some("test_profile".into()),
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::paytabs_connector::PayTabsConnector::new(&paytabs_config)));

        let mamo_config = ConnectorConfig {
            api_key: Some("test_api_key".into()),
            secret_key: None,
            merchant_id: None,
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::mamo_connector::MamoConnector::new(&mamo_config)));

        let aps_config = ConnectorConfig {
            api_key: Some("test_access_code".into()),
            secret_key: None,
            merchant_id: Some("test_merchant_id".into()),
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::from([
                ("sha_request_phrase".into(), "test_phrase".into()),
                ("sha_response_phrase".into(), "test_response_phrase".into()),
            ]),
        };
        registry.register(Box::new(super::amazon_ps_connector::AmazonPsConnector::new(&aps_config)));

        let aani_config = ConnectorConfig {
            api_key: Some("test_client_id".into()),
            secret_key: Some("test_client_secret".into()),
            merchant_id: None,
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: HashMap::new(),
        };
        registry.register(Box::new(super::aani_connector::AaniConnector::new(&aani_config)));

        // Register mock connectors for testing
        registry.register(Box::new(super::mock_connectors::MockNetworkIntlConnector::new("sandbox")));
        registry.register(Box::new(super::mock_connectors::MockCheckoutComConnector::new("sandbox")));
        registry.register(Box::new(super::mock_connectors::MockTelrConnector::new("sandbox")));

        // Register India payment gateway connectors
        let billdesk_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: Some("test_secret".into()), merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::billdesk_connector::BilldeskConnector::new(&billdesk_config)));

        let cashfree_config = ConnectorConfig { api_key: Some("test_client_id".into()), secret_key: Some("test_client_secret".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::cashfree_connector::CashfreeConnector::new(&cashfree_config)));

        let payu_config = ConnectorConfig { api_key: Some("test_merchant_key".into()), secret_key: Some("test_salt".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::payu_connector::PayuConnector::new(&payu_config)));

        let phonepe_config = ConnectorConfig { api_key: Some("test_client_id".into()), secret_key: Some("test_client_secret".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::phonepe_connector::PhonepeConnector::new(&phonepe_config)));

        let razorpay_config = ConnectorConfig { api_key: Some("rzp_test_placeholder".into()), secret_key: Some("test_key_secret".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::razorpay_connector::RazorpayConnector::new(&razorpay_config)));

        let ccavenue_config = ConnectorConfig { api_key: Some("test_access_code".into()), secret_key: Some("test_working_key".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::ccavenue_connector::CcavenueConnector::new(&ccavenue_config)));

        let paytm_config = ConnectorConfig { api_key: None, secret_key: Some("test_merchant_key".into()), merchant_id: Some("test_merchant_id".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::paytm_connector::PaytmConnector::new(&paytm_config)));

        let juspay_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::juspay_connector::JuspayConnector::new(&juspay_config)));

        let instamojo_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: Some("test_auth_token".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::instamojo_connector::InstamojoConnector::new(&instamojo_config)));

        let easebuzz_config = ConnectorConfig { api_key: Some("test_merchant_key".into()), secret_key: Some("test_merchant_iv".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::easebuzz_connector::EasebuzzConnector::new(&easebuzz_config)));

        // Register India bank-specific connectors
        let icici_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::icici_connector::IciciConnector::new(&icici_config)));

        let hdfc_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::hdfc_connector::HdfcConnector::new(&hdfc_config)));

        let axis_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::axis_connector::AxisConnector::new(&axis_config)));

        let sbi_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::sbi_connector::SbiConnector::new(&sbi_config)));

        // Register UPI ecosystem connector
        let upi_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::from([("vpa".into(), "merchant@upi".into())]) };
        registry.register(Box::new(super::upi_connector::UpiConnector::new(&upi_config)));

        // Register additional India connectors
        let pinelabs_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::pinelabs_connector::PineLabsConnector::new(&pinelabs_config)));

        let worldline_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::worldline_connector::WorldlineConnector::new(&worldline_config)));

        let zaakpay_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::zaakpay_connector::ZaakpayConnector::new(&zaakpay_config)));

        let payglocal_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::payglocal_connector::PayGlocalConnector::new(&payglocal_config)));

        // Register enterprise connectors
        let adyen_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::adyen_connector::AdyenConnector::new(&adyen_config)));

        let paypal_config = ConnectorConfig { api_key: Some("test_client_id".into()), secret_key: Some("test_client_secret".into()), merchant_id: None, store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::paypal_connector::PaypalConnector::new(&paypal_config)));

        // Register remaining India connectors
        let kotak_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::kotak_connector::KotakConnector::new(&kotak_config)));

        let yesbank_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::yesbank_connector::YesbankConnector::new(&yesbank_config)));

        let indusind_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::indusind_connector::IndusindConnector::new(&indusind_config)));

        let mswipe_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::mswipe_connector::MswipeConnector::new(&mswipe_config)));

        let fampay_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::fampay_connector::FampayConnector::new(&fampay_config)));

        let zestmoney_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::zestmoney_connector::ZestmoneyConnector::new(&zestmoney_config)));

        let direcpay_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::direcpay_connector::DirecpayConnector::new(&direcpay_config)));

        let atom_config = ConnectorConfig { api_key: Some("test_api_key".into()), secret_key: None, merchant_id: Some("test_merchant".into()), store_id: None, environment: "sandbox".into(), additional_fields: HashMap::new() };
        registry.register(Box::new(super::atom_connector::AtomConnector::new(&atom_config)));

        registry
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
