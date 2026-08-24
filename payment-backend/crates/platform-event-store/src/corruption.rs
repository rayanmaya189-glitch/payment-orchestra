//! Corruption detection for event store integrity.
//! Computes and verifies SHA-256 checksums over event payloads.

use sha2::{Sha256, Digest};

/// Compute SHA-256 checksum for an event payload.
pub fn compute_checksum(payload: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.finalize().to_vec()
}

/// Verify an event payload against a stored checksum.
/// Returns true if the checksum matches or if no checksum is stored (backward compat).
pub fn verify_checksum(payload: &[u8], stored_checksum: Option<&[u8]>) -> bool {
    match stored_checksum {
        Some(expected) => {
            let computed = compute_checksum(payload);
            computed == expected
        }
        None => {
            // No checksum stored — legacy event, skip verification
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_is_deterministic() {
        let payload = b"hello world";
        let c1 = compute_checksum(payload);
        let c2 = compute_checksum(payload);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_checksum_is_32_bytes() {
        let payload = b"any data";
        let checksum = compute_checksum(payload);
        assert_eq!(checksum.len(), 32);
    }

    #[test]
    fn test_verify_valid_checksum() {
        let payload = b"test payload";
        let checksum = compute_checksum(payload);
        assert!(verify_checksum(payload, Some(&checksum)));
    }

    #[test]
    fn test_verify_invalid_checksum() {
        let payload = b"test payload";
        let wrong = compute_checksum(b"different payload");
        assert!(!verify_checksum(payload, Some(&wrong)));
    }

    #[test]
    fn test_verify_no_checksum_is_valid() {
        assert!(verify_checksum(b"anything", None));
    }
}
