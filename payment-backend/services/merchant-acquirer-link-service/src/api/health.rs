#[allow(dead_code)]
pub struct LinkHealth;

#[allow(dead_code)]
impl LinkHealth {
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
