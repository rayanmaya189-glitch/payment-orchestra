use platform_health::liveness;

#[allow(dead_code)]
pub struct IamHealth;

#[allow(dead_code)]
impl IamHealth {
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
