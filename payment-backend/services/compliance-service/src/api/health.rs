#[allow(dead_code)]
pub struct ComplianceHealth;

#[allow(dead_code)]
impl ComplianceHealth {
    pub fn new() -> Self {
        Self
    }

    pub fn check_liveness(&self) -> bool {
        true
    }

    pub fn check_readiness(&self, db_connected: bool) -> bool {
        db_connected
    }
}
