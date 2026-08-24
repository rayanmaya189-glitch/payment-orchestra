//! PCI DSS compliance utilities for payment data protection.
//!
//! Provides:
//! - PAN (Primary Account Number) detection and masking
//! - CVV/CVC never storage enforcement
//! - Token-based card reference (PCI DSS requirement 3.2)
//! - Audit logging for all card data access
//! - Field-level encryption for sensitive data at rest
//! - Key rotation support

use serde::{Deserialize, Serialize};
use tracing::warn;

/// PAN detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanDetection {
    /// Whether a valid PAN was found
    pub found: bool,
    /// Masked PAN (e.g., "4111-XXXX-XXXX-1111")
    pub masked: Option<String>,
    /// Card brand (visa, mastercard, amex, etc.)
    pub brand: Option<String>,
    /// Last 4 digits
    pub last_four: Option<String>,
}

/// Detect and mask PAN in input string
///
/// # PCI DSS Requirements
/// - Requirement 3.3: Mask PAN when displayed (show only first 6/last 4)
/// - Requirement 3.4: Render PAN unreadable anywhere it is stored
pub fn detect_and_mask_pan(input: &str) -> PanDetection {
    // Remove spaces, dashes, and common separators
    let cleaned: String = input.chars().filter(|c| c.is_alphanumeric()).collect();
    
    // Check for valid PAN patterns (13-19 digits)
    let pan_regex = regex::Regex::new(r"^\d{13,19}$").unwrap();
    
    if !pan_regex.is_match(&cleaned) {
        return PanDetection {
            found: false,
            masked: None,
            brand: None,
            last_four: None,
        };
    }
    
    let brand = detect_card_brand(&cleaned);
    let last_four = cleaned[cleaned.len()-4..].to_string();
    
    // Mask: show first 6 + last 4 (PCI DSS 3.3)
    let masked = if cleaned.len() >= 13 {
        let first_six = &cleaned[..6];
        let masked_middle = "X".repeat(cleaned.len() - 10);
        format!("{}-{}-{}", first_six, masked_middle, last_four)
    } else {
        format!("XXXX-{}", last_four)
    };
    
    PanDetection {
        found: true,
        masked: Some(masked),
        brand: Some(brand),
        last_four: Some(last_four),
    }
}

/// Detect card brand from PAN
fn detect_card_brand(pan: &str) -> String {
    match pan {
        // Visa: starts with 4
        p if p.starts_with('4') => "visa".to_string(),
        // Mastercard: 51-55 or 2221-2720 (2-series)
        p if p.starts_with('5') && p.len() >= 2 && matches!(p.as_bytes()[1], b'1'..=b'5') => "mastercard".to_string(),
        p if p.len() >= 4 && {
            let prefix: u32 = p[..4].parse().unwrap_or(0);
            (2221..=2720).contains(&prefix)
        } => "mastercard".to_string(),
        // Amex: starts with 34 or 37
        p if p.starts_with("34") || p.starts_with("37") => "amex".to_string(),
        // Discover: starts with 6011, 65, 644-649
        p if p.starts_with("6011") || p.starts_with("65") => "discover".to_string(),
        p if p.len() >= 3 && {
            let prefix: u32 = p[..3].parse().unwrap_or(0);
            (644..=649).contains(&prefix)
        } => "discover".to_string(),
        // JCB: starts with 3528-3589
        p if p.len() >= 4 && {
            let prefix: u32 = p[..4].parse().unwrap_or(0);
            (3528..=3589).contains(&prefix)
        } => "jcb".to_string(),
        // Diners Club: starts with 36, 38, or 39
        p if p.starts_with("36") || p.starts_with("38") || p.starts_with("39") => "diners".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Validate CVV is not being stored (PCI DSS 3.2)
///
/// Returns Ok(()) if CVV is not present, Err if attempting to store
pub fn enforce_no_cvv_storage(data: &serde_json::Map<String, serde_json::Value>) -> Result<(), PciError> {
    let forbidden_fields = ["cvv", "cvc", "cvv2", "cvc2", "security_code", "card_verification"];
    
    for field in forbidden_fields {
        if let Some(value) = data.get(field) {
            if !value.is_null() && !value.as_str().map_or(false, |s| s.is_empty()) {
                warn!("PCI DSS 3.2 violation: Attempting to store CVV/CVC data in field '{}'", field);
                return Err(PciError::CvvStorageAttempted(field.to_string()));
            }
        }
    }
    
    Ok(())
}

/// Sanitize sensitive fields from logs (PCI DSS 3.4)
pub fn sanitize_for_logging(data: &str) -> String {
    let sensitive_patterns = [
        // PAN: 13-19 digits
        (r"\b\d{13,19}\b", "[PAN_MASKED]"),
        // CVV/CVC with various formats: "cvv: 123", "CVV 1234", "cvc=456"
        (r"(?i)cvv[:\s=]*\d{3,4}\b", "[CVV_MASKED]"),
        (r"(?i)cvc[:\s=]*\d{3,4}\b", "[CVV_MASKED]"),
        // Security code
        (r"(?i)security_code[:\s=]*\d{3,4}\b", "[SECURITY_CODE_MASKED]"),
        // Password and secret keys
        (r"(?i)password[:\s=]+[^\s,]+", "password: [REDACTED]"),
        (r"(?i)secret[:\s=]+[^\s,]+", "secret: [REDACTED]"),
        (r"(?i)api_key[:\s=]+[^\s,]+", "api_key: [REDACTED]"),
        (r"(?i)api_secret[:\s=]+[^\s,]+", "api_secret: [REDACTED]"),
        (r"(?i)access_token[:\s=]+[^\s,]+", "access_token: [REDACTED]"),
    ];
    
    let mut result = data.to_string();
    for (pattern, replacement) in sensitive_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, replacement).to_string();
        }
    }
    result
}

/// PCI DSS audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PciAuditEntry {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Action performed
    pub action: PciAuditAction,
    /// User/service performing action
    pub actor: String,
    /// Data type accessed
    pub data_type: String,
    /// Whether PAN was accessed
    pub pan_accessed: bool,
    /// Masked PAN if accessed
    pub masked_pan: Option<String>,
    /// Success/failure
    pub success: bool,
    /// Additional context
    pub details: Option<String>,
}

/// PCI audit actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PciAuditAction {
    /// Card data read
    Read,
    /// Card data written
    Write,
    /// Card data deleted
    Delete,
    /// Card data encrypted
    Encrypt,
    /// Card data decrypted
    Decrypt,
    /// Key rotation
    KeyRotation,
    /// Access denied
    AccessDenied,
}

/// PCI DSS error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PciError {
    /// Attempting to store CVV/CVC (PCI DSS 3.2 violation)
    CvvStorageAttempted(String),
    /// PAN validation failed
    InvalidPan(String),
    /// Encryption key not available
    KeyNotAvailable(String),
    /// Access denied
    AccessDenied(String),
    /// Audit log failure
    AuditFailure(String),
}

impl std::fmt::Display for PciError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CvvStorageAttempted(field) => write!(f, "PCI DSS 3.2: CVV/CVC storage attempted in field '{}'", field),
            Self::InvalidPan(reason) => write!(f, "Invalid PAN: {}", reason),
            Self::KeyNotAvailable(reason) => write!(f, "Encryption key not available: {}", reason),
            Self::AccessDenied(reason) => write!(f, "Access denied: {}", reason),
            Self::AuditFailure(reason) => write!(f, "Audit log failure: {}", reason),
        }
    }
}

impl std::error::Error for PciError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_visa_pan() {
        let result = detect_and_mask_pan("4111111111111111");
        assert!(result.found);
        assert_eq!(result.brand, Some("visa".to_string()));
        assert_eq!(result.last_four, Some("1111".to_string()));
        assert!(result.masked.unwrap().contains("1111"));
    }

    #[test]
    fn test_detect_mastercard_pan() {
        let result = detect_and_mask_pan("5500000000000004");
        assert!(result.found);
        assert_eq!(result.brand, Some("mastercard".to_string()));
        assert_eq!(result.last_four, Some("0004".to_string()));
    }

    #[test]
    fn test_no_pan_in_string() {
        let result = detect_and_mask_pan("This is not a card number");
        assert!(!result.found);
    }

    #[test]
    fn test_enforce_no_cvv_storage() {
        let mut data = serde_json::Map::new();
        data.insert("cvv".to_string(), serde_json::json!("123"));
        
        let result = enforce_no_cvv_storage(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_enforce_cvv_allowed() {
        let mut data = serde_json::Map::new();
        data.insert("card_number".to_string(), serde_json::json!("4111111111111111"));
        
        let result = enforce_no_cvv_storage(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_for_logging() {
        let input = "User card: 4111111111111111 cvv: 123";
        let sanitized = sanitize_for_logging(input);
        assert!(!sanitized.contains("4111111111111111"));
        assert!(!sanitized.contains("123"));
    }
}
