//! Query types for connector-gateway.

#[derive(Debug, Clone)]
pub struct ConnectorInfo {
    pub connector_id: String,
    pub capabilities: String, // JSON-serialized
    pub settlement_cycle: String,
    pub test_card_count: usize,
}
