/// Field-level encryption for PII data (email, phone, etc.)
///
/// Uses AES-256-GCM for authenticated encryption.
/// The DEK (Data Encryption Key) is provided via environment variable.
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use aes_gcm::aead::rand_core::RngCore;
use platform_error::PlatformError;

/// Encrypt plaintext using AES-256-GCM.
///
/// Returns: nonce (12 bytes) + ciphertext, base64-encoded.
pub fn encrypt_field(plaintext: &str, key: &[u8; 32]) -> Result<String, PlatformError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| PlatformError::Internal(format!("Cipher init failed: {e}")))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| PlatformError::Internal(format!("Encryption failed: {e}")))?;

    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(hex::encode(result))
}

/// Decrypt ciphertext using AES-256-GCM.
///
/// Input: nonce (12 bytes) + ciphertext, base64-encoded.
pub fn decrypt_field(encrypted: &str, key: &[u8; 32]) -> Result<String, PlatformError> {
    let data = hex::decode(encrypted)
        .map_err(|e| PlatformError::Internal(format!("Invalid encrypted data: {e}")))?;

    if data.len() < 12 {
        return Err(PlatformError::Internal("Encrypted data too short".to_string()));
    }

    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| PlatformError::Internal(format!("Cipher init failed: {e}")))?;

    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| PlatformError::Internal(format!("Decryption failed: {e}")))?;

    String::from_utf8(plaintext)
        .map_err(|e| PlatformError::Internal(format!("Invalid UTF-8: {e}")))
}

/// Derive a 256-bit key from a passphrase using Argon2id with production parameters.
///
/// Production parameters (OWASP recommended for password hashing):
/// - Memory: 64 MiB (65536 KiB)
/// - Iterations: 3
/// - Parallelism: 4 threads
/// - Output: 32 bytes (256 bits)
pub fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    use argon2::password_hash::PasswordHasher;
    use argon2::{Argon2, Algorithm, Version, Params};

    // Production-tuned Argon2id parameters (OWASP 2024 recommendations)
    let params = Params::new(65536, 3, 4, Some(32))
        .expect("Valid Argon2 parameters");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut output = [0u8; 32];
    argon2.hash_password_into(passphrase.as_bytes(), salt, &mut output)
        .expect("Key derivation failed");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = "test@example.com";
        let encrypted = encrypt_field(plaintext, &key).unwrap();
        let decrypted = decrypt_field(&encrypted, &key).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_different_each_time() {
        let key = [0x42u8; 32];
        let plaintext = "test@example.com";
        let enc1 = encrypt_field(plaintext, &key).unwrap();
        let enc2 = encrypt_field(plaintext, &key).unwrap();
        // Different nonces produce different ciphertext
        assert_ne!(enc1, enc2);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = [0x42u8; 32];
        let key2 = [0x43u8; 32];
        let encrypted = encrypt_field("test", &key1).unwrap();
        assert!(decrypt_field(&encrypted, &key2).is_err());
    }
}
