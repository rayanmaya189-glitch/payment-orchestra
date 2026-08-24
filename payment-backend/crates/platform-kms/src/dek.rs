/// Data Encryption Key management.
pub struct DekManager {
    pub rotation_period_days: u32,
}

impl Default for DekManager {
    fn default() -> Self {
        Self { rotation_period_days: 90 }
    }
}
