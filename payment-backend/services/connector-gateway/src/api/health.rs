//! Connector-gateway health check.

#[allow(dead_code)]
pub struct ConnectorGatewayHealth;

#[allow(dead_code)]
impl ConnectorGatewayHealth {
    #[allow(dead_code)]
    pub fn check_liveness(&self) -> bool {
        true
    }

    #[allow(dead_code)]
    pub fn check_readiness(&self) -> bool {
        true
    }
}
