use platform_health::liveness;

#[allow(dead_code)]
pub struct IamHealth;

#[allow(dead_code)]
impl Default for IamHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl IamHealth {
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
