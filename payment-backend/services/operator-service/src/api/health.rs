use platform_health::liveness;

#[allow(dead_code)]
pub struct OperatorHealth;

#[allow(dead_code)]
impl OperatorHealth {
    pub fn new() -> Self {
        Self
    }

    pub fn check_liveness(&self) -> bool {
        liveness::liveness()
    }

    pub fn check_readiness(&self, db_connected: bool) -> bool {
        db_connected
    }
}
