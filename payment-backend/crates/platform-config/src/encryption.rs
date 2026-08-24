/// Placeholder for encryption configuration helpers.
/// Full implementation lives in platform-kms.
pub struct EncryptionConfig {
    pub kek_id: String,
    pub algorithm: &'static str,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self { kek_id: "platform-kek-1".into(), algorithm: "AES-256-GCM" }
    }
}
