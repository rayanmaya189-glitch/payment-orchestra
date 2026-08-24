#[allow(dead_code)]
pub struct LinkHealth;

#[allow(dead_code)]
impl Default for LinkHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkHealth {
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
