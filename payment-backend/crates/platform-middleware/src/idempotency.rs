use uuid::Uuid;

pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    pub fn new(key: &str) -> Result<Self, String> {
        if key.len() < 16 || key.len() > 256 {
            return Err("Idempotency key must be 16-256 characters".into());
        }
        Ok(Self(key.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn generate_idempotency_key() -> String {
    Uuid::now_v7().to_string()
}
