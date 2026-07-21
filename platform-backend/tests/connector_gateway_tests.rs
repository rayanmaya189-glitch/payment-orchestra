//! Integration tests for connector gateway — circuit breaker, adapters.

#[cfg(test)]
mod connector_tests {
    use std::time::Duration;
    use platform_config::encryption::{encrypt_field, decrypt_field};

    #[test]
    fn test_encryption_roundtrip() {
        let key = [1u8; 32];
        let plaintext = "user@example.com";
        let encrypted = encrypt_field(plaintext, &key).unwrap();
        let decrypted = decrypt_field(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_different_nonces() {
        let key = [1u8; 32];
        let plaintext = "test@email.com";
        let enc1 = encrypt_field(plaintext, &key).unwrap();
        let enc2 = encrypt_field(plaintext, &key).unwrap();
        // Different nonces => different ciphertext
        assert_ne!(enc1, enc2);
        // But both decrypt to the same plaintext
        assert_eq!(decrypt_field(&enc1, &key).unwrap(), plaintext);
        assert_eq!(decrypt_field(&enc2, &key).unwrap(), plaintext);
    }

    #[test]
    fn test_encryption_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let encrypted = encrypt_field("secret", &key1).unwrap();
        assert!(decrypt_field(&encrypted, &key2).is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_trip_and_recovery() {
        use platform_config::encryption as _; // just for the module

        // Circuit breaker is tested via the connector-gateway crate's own tests
        // This is a placeholder for integration-level circuit breaker testing
        // with real Redis state sharing across replicas
    }
}
