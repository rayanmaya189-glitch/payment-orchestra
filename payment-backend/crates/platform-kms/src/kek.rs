/// Key Encryption Key management.
pub struct KekManager {
    pub kek_id: String,
    pub current_version: String,
}

impl KekManager {
    pub fn new(kek_id: &str) -> Self {
        Self { kek_id: kek_id.to_string(), current_version: "v1".to_string() }
    }
}
