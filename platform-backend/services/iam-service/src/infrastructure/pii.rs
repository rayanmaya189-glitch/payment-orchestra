/// PII encryption helper for iam-service.
///
/// Encrypts/decrypts email fields before storing in database.
use platform_config::encryption::{encrypt_field, decrypt_field};
use platform_error::PlatformError;

/// Get the encryption key from environment or config.
fn get_encryption_key() -> Result<[u8; 32], PlatformError> {
    let key_hex = std::env::var("PLATFORM__AUTH__ENCRYPTION_KEY")
        .unwrap_or_else(|_| "0000000000000000000000000000000000000000000000000000000000000000".to_string());

    let key_bytes = hex::decode(&key_hex)
        .map_err(|e| PlatformError::Internal(format!("Invalid encryption key: {e}")))?;

    if key_bytes.len() != 32 {
        return Err(PlatformError::Internal("Encryption key must be 32 bytes".to_string()));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    Ok(key)
}

/// Encrypt an email field for storage.
pub fn encrypt_email(email: &str) -> Result<String, PlatformError> {
    let key = get_encryption_key()?;
    encrypt_field(email, &key)
}

/// Decrypt an email field from storage.
pub fn decrypt_email(encrypted: &str) -> Result<String, PlatformError> {
    let key = get_encryption_key()?;
    decrypt_field(encrypted, &key)
}

/// Check if a string looks like an encrypted value (hex-encoded).
pub fn is_encrypted(value: &str) -> bool {
    // Encrypted values are hex-encoded and at least 24 chars (12 nonce + 12+ ciphertext)
    value.len() >= 24 && value.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_encrypted() {
        assert!(is_encrypted("0123456789abcdef0123456789abcdef0123456789abcdef"));
        assert!(!is_encrypted("user@example.com"));
        assert!(!is_encrypted("short"));
    }
}
