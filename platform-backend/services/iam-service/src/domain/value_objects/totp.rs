/// TOTP (Time-based One-Time Password) implementation per RFC 6238.
///
/// Generates and verifies 6-digit TOTP codes for MFA enrollment.
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

/// Generate a new TOTP secret (160-bit, base32 encoded).
pub fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..20).map(|_| rng.gen()).collect();
    base32_encode(&bytes)
}

/// Generate a TOTP URI for QR code scanning.
pub fn generate_uri(secret: &str, email: &str, issuer: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&digits=6&period=30",
        issuer, email, secret, issuer
    )
}

/// Verify a TOTP code against a secret.
pub fn verify_totp(secret: &str, code: &str) -> bool {
    let secret_bytes = match base32_decode(secret) {
        Some(b) => b,
        None => return false,
    };

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check current and adjacent time steps (±1 window)
    for offset in [-1i64, 0, 1] {
        let time_step = ((current_time as i64 / 30) + offset) as u64;
        let expected = generate_code(&secret_bytes, time_step);
        if expected == code {
            return true;
        }
    }
    false
}

/// Generate a 6-digit TOTP code for a given time step.
fn generate_code(secret: &[u8], time_step: u64) -> String {
    let mut msg = [0u8; 8];
    msg.copy_from_slice(&time_step.to_be_bytes());

    let mut mac = HmacSha1::new_from_slice(secret).expect("HMAC key creation failed");
    mac.update(&msg);
    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0x0f) as usize;
    let code = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);

    format!("{:06}", code % 1_000_000)
}

/// Base32 encode (RFC 4648).
fn base32_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut buffer = 0u32;
    let mut bits_left = 0;

    for &byte in data {
        buffer = (buffer << 8) | (byte as u32);
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            result.push(CHARSET[((buffer >> bits_left) & 0x1f) as usize] as char);
        }
    }

    if bits_left > 0 {
        buffer <<= 5 - bits_left;
        result.push(CHARSET[(buffer & 0x1f) as usize] as char);
    }

    // Add padding
    while result.len() % 8 != 0 {
        result.push('=');
    }

    result
}

/// Base32 decode (RFC 4648).
fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let mut buffer = 0u32;
    let mut bits_left = 0;

    for c in input.chars() {
        let val = match c {
            'A'..='Z' => c as u8 - b'A',
            '2'..='7' => c as u8 - b'2' + 26,
            'a'..='z' => c as u8 - b'a',
            _ => return None,
        };

        buffer = (buffer << 5) | (val as u32);
        bits_left += 5;

        if bits_left >= 8 {
            bits_left -= 8;
            result.push((buffer >> bits_left) as u8);
        }
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let secret = generate_secret();
        assert!(!secret.is_empty());
        assert!(secret.len() >= 32); // 160 bits = 32 base32 chars
    }

    #[test]
    fn test_generate_uri() {
        let uri = generate_uri("JBSWY3DPEHPK3PXP", "test@example.com", "PaymentOrchestra");
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("secret=JBSWY3DPEHPK3PXP"));
        assert!(uri.contains("issuer=PaymentOrchestra"));
    }

    #[test]
    fn test_verify_totp_valid() {
        let secret = generate_secret();
        let time_step = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() / 30;
        let code = generate_code(&base32_decode(&secret).unwrap(), time_step);
        assert!(verify_totp(&secret, &code));
    }

    #[test]
    fn test_verify_totp_invalid() {
        let secret = generate_secret();
        assert!(!verify_totp(&secret, "000000"));
        assert!(!verify_totp(&secret, "123456"));
    }

    #[test]
    fn test_base32_roundtrip() {
        let original = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        let encoded = base32_encode(&original);
        let decoded = base32_decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }
}
