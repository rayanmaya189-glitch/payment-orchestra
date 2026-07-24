use platform_health::liveness;

#[allow(dead_code)]
pub struct OperatorHealth;

#[allow(dead_code)]
impl Default for OperatorHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorHealth {
    pub fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    pub fn check_liveness(&self) -> bool {
        liveness::liveness()
    }

    #[allow(dead_code)]
    pub fn check_readiness(&self, db_connected: bool) -> bool {
        db_connected
    }
}
