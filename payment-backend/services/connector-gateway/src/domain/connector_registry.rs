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

        // Register mock connectors for testing
        registry.register(Box::new(super::mock_connectors::MockNetworkIntlConnector::new("sandbox")));
        registry.register(Box::new(super::mock_connectors::MockCheckoutComConnector::new("sandbox")));
        registry.register(Box::new(super::mock_connectors::MockTelrConnector::new("sandbox")));

        registry
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
