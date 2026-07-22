//! Connector Gateway — Anti-Corruption Layer for acquirer integrations.
//! Hosts the AcquirerConnector trait, ConnectorRegistry, GatewayProfile lifecycle,
//! circuit breakers, and mock connector implementations for testing.

use connector_gateway::domain;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("connector-gateway starting...");

    // Initialize connector registry with mock connectors
    let mut registry = domain::ConnectorRegistry::new();
    registry.register(Box::new(domain::mocks::MockNetworkIntlConnector::new("sandbox")));
    registry.register(Box::new(domain::mocks::MockCheckoutComConnector::new("sandbox")));
    registry.register(Box::new(domain::mocks::MockTelrConnector::new("sandbox")));
    tracing::info!(connectors = ?registry.list_ids(), "Connector registry initialized");

    tracing::info!("connector-gateway ready");

    Ok(())
}
