/// Strips sensitive fields (PAN, credentials, tokens) from log metadata.
pub fn scrub_sensitive_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for key in ["password", "secret", "token", "api_key", "api_secret", "cvv", "pan", "card_number"].iter() {
                if map.contains_key(*key) {
                    map.insert(key.to_string(), serde_json::Value::String("[REDACTED]".into()));
                }
            }
            for val in map.values_mut() {
                scrub_sensitive_fields(val);
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                scrub_sensitive_fields(val);
            }
        }
        _ => {}
    }
}
