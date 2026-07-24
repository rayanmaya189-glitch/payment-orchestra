#[allow(dead_code)]
pub struct ComplianceHealth;

#[allow(dead_code)]
impl Default for ComplianceHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceHealth {
    pub fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    pub fn check_liveness(&self) -> bool {
        true
    }

    #[allow(dead_code)]
    pub fn check_readiness(&self, db_connected: bool) -> bool {
        db_connected
    }
}
