//! Connector-gateway health check.

#[allow(dead_code)]
pub struct ConnectorGatewayHealth;

#[allow(dead_code)]
impl ConnectorGatewayHealth {
    pub fn check_liveness(&self) -> bool {
        true
    }

    pub fn check_readiness(&self) -> bool {
        true
    }
}
