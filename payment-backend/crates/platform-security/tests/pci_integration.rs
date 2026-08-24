//! Integration tests for PCI DSS compliance utilities.
//!
//! These tests verify:
//! - PAN detection and masking for all card brands
//! - CVV/CVC storage enforcement
//! - Log sanitization for sensitive data
//! - Audit logging structure
//! - Error handling and edge cases

use platform_security::pci::*;

// ============================================================================
// PAN Detection Tests - Visa
// ============================================================================

#[test]
fn test_detect_visa_pan_16_digits() {
    let result = detect_and_mask_pan("4111111111111111");
    
    assert!(result.found, "Should detect valid Visa PAN");
    assert_eq!(result.brand, Some("visa".to_string()));
    assert_eq!(result.last_four, Some("1111".to_string()));
    
    let masked = result.masked.unwrap();
    assert!(masked.starts_with("411111"), "Should show first 6 digits");
    assert!(masked.ends_with("1111"), "Should show last 4 digits");
    assert!(masked.contains("XXXX"), "Should mask middle digits");
}

#[test]
fn test_detect_visa_pan_with_dashes() {
    let result = detect_and_mask_pan("4111-1111-1111-1111");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("visa".to_string()));
    assert_eq!(result.last_four, Some("1111".to_string()));
}

#[test]
fn test_detect_visa_pan_with_spaces() {
    let result = detect_and_mask_pan("4111 1111 1111 1111");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("visa".to_string()));
}

#[test]
fn test_detect_visa_pan_13_digits() {
    let result = detect_and_mask_pan("4222222222222");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("visa".to_string()));
    assert_eq!(result.last_four, Some("2222".to_string()));
}

// ============================================================================
// PAN Detection Tests - Mastercard
// ============================================================================

#[test]
fn test_detect_mastercard_pan() {
    let result = detect_and_mask_pan("5500000000000004");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("mastercard".to_string()));
    assert_eq!(result.last_four, Some("0004".to_string()));
}

#[test]
fn test_detect_mastercard_pan_2series() {
    // Mastercard 2-series
    let result = detect_and_mask_pan("2223000000000007");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("mastercard".to_string()));
}

// ============================================================================
// PAN Detection Tests - American Express
// ============================================================================

#[test]
fn test_detect_amex_pan_15_digits() {
    let result = detect_and_mask_pan("378282246310005");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("amex".to_string()));
    assert_eq!(result.last_four, Some("0005".to_string()));
}

#[test]
fn test_detect_amex_pan_with_dashes() {
    let result = detect_and_mask_pan("3782-822463-10005");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("amex".to_string()));
}

// ============================================================================
// PAN Detection Tests - Discover
// ============================================================================

#[test]
fn test_detect_discover_pan_6011() {
    let result = detect_and_mask_pan("6011111111111117");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("discover".to_string()));
    assert_eq!(result.last_four, Some("1117".to_string()));
}

#[test]
fn test_detect_discover_pan_65() {
    let result = detect_and_mask_pan("6500000000000002");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("discover".to_string()));
}

// ============================================================================
// PAN Detection Tests - JCB
// ============================================================================

#[test]
fn test_detect_jcb_pan() {
    let result = detect_and_mask_pan("3530111333300000");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("jcb".to_string()));
}

// ============================================================================
// PAN Detection Tests - Diners Club
// ============================================================================

#[test]
fn test_detect_diners_pan() {
    let result = detect_and_mask_pan("36110361103612");
    
    assert!(result.found);
    assert_eq!(result.brand, Some("diners".to_string()));
}

// ============================================================================
// PAN Detection Tests - Invalid Inputs
// ============================================================================

#[test]
fn test_no_pan_in_text() {
    let result = detect_and_mask_pan("This is not a card number");
    
    assert!(!result.found);
    assert!(result.masked.is_none());
    assert!(result.brand.is_none());
    assert!(result.last_four.is_none());
}

#[test]
fn test_empty_string() {
    let result = detect_and_mask_pan("");
    
    assert!(!result.found);
}

#[test]
fn test_too_short_number() {
    let result = detect_and_mask_pan("411111111111");
    
    assert!(!result.found, "12 digits should not be detected as PAN");
}

#[test]
fn test_too_long_number() {
    let result = detect_and_mask_pan("411111111111111112345");
    
    // Should still detect if within 13-19 digit range after cleaning
    // 21 digits is too long
    assert!(!result.found);
}

#[test]
fn test_alphanumeric混合() {
    // Note: The function removes non-alphanumeric chars, then checks if result is 13-19 digits
    // This test verifies that non-digit characters are stripped correctly
    let result = detect_and_mask_pan("4111-1111-1111-1111");
    
    // After removing dashes, should be 16 digits
    assert!(result.found);
    assert_eq!(result.brand, Some("visa".to_string()));
}

// ============================================================================
// PAN Masking Format Tests
// ============================================================================

#[test]
fn test_masking_format_visa() {
    let result = detect_and_mask_pan("4111111111111111");
    let masked = result.masked.unwrap();
    
    // Format: XXXXX...XXXX (first 6 + XXXX... + last 4)
    let parts: Vec<&str> = masked.split('-').collect();
    assert_eq!(parts.len(), 3, "Masked PAN should have 3 parts separated by dashes");
    assert_eq!(parts[0].len(), 6, "First part should be 6 digits");
    assert_eq!(parts[2].len(), 4, "Last part should be 4 digits");
    assert!(parts[1].chars().all(|c| c == 'X'), "Middle part should be all X's");
}

#[test]
fn test_masking_preserves_first_six() {
    let result = detect_and_mask_pan("4111111111111111");
    let masked = result.masked.unwrap();
    
    assert!(masked.starts_with("411111"), "Should preserve first 6 digits");
}

#[test]
fn test_masking_preserves_last_four() {
    let result = detect_and_mask_pan("4111111111111111");
    let masked = result.masked.unwrap();
    
    assert!(masked.ends_with("1111"), "Should preserve last 4 digits");
}

// ============================================================================
// CVV Storage Enforcement Tests
// ============================================================================

#[test]
fn test_enforce_no_cvv_storage_cvv() {
    let mut data = serde_json::Map::new();
    data.insert("cvv".to_string(), serde_json::json!("123"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_err(), "Should reject CVV storage");
    
    match result.unwrap_err() {
        PciError::CvvStorageAttempted(field) => {
            assert_eq!(field, "cvv");
        }
        _ => panic!("Expected CvvStorageAttempted error"),
    }
}

#[test]
fn test_enforce_no_cvv_storage_cvc() {
    let mut data = serde_json::Map::new();
    data.insert("cvc".to_string(), serde_json::json!("456"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_err());
}

#[test]
fn test_enforce_no_cvv_storage_cvv2() {
    let mut data = serde_json::Map::new();
    data.insert("cvv2".to_string(), serde_json::json!("789"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_err());
}

#[test]
fn test_enforce_no_cvv_storage_security_code() {
    let mut data = serde_json::Map::new();
    data.insert("security_code".to_string(), serde_json::json!("123"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_err());
}

#[test]
fn test_enforce_no_cvv_storage_card_verification() {
    let mut data = serde_json::Map::new();
    data.insert("card_verification".to_string(), serde_json::json!("456"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_err());
}

#[test]
fn test_enforce_cvv_allowed_empty() {
    let mut data = serde_json::Map::new();
    data.insert("cvv".to_string(), serde_json::json!(""));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_ok(), "Empty CVV should be allowed");
}

#[test]
fn test_enforce_cvv_allowed_null() {
    let mut data = serde_json::Map::new();
    data.insert("cvv".to_string(), serde_json::json!(null));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_ok(), "Null CVV should be allowed");
}

#[test]
fn test_enforce_no_cvv_card_number_allowed() {
    let mut data = serde_json::Map::new();
    data.insert("card_number".to_string(), serde_json::json!("4111111111111111"));
    data.insert("exp_month".to_string(), serde_json::json!("12"));
    data.insert("exp_year".to_string(), serde_json::json!("2030"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_ok(), "Card number and expiry should be allowed");
}

#[test]
fn test_enforce_multiple_cvv_fields() {
    let mut data = serde_json::Map::new();
    data.insert("cvv".to_string(), serde_json::json!("123"));
    data.insert("cvc".to_string(), serde_json::json!("456"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_err(), "Should reject when any CVV field is present");
}

// ============================================================================
// Log Sanitization Tests
// ============================================================================

#[test]
fn test_sanitize_pan_in_logs() {
    let input = "User card: 4111111111111111 processed";
    let sanitized = sanitize_for_logging(input);
    
    assert!(!sanitized.contains("4111111111111111"), "PAN should be masked");
    assert!(sanitized.contains("[PAN_MASKED]"), "Should contain PAN mask");
}

#[test]
fn test_sanitize_cvv_in_logs() {
    let input = "CVV: 123 validated";
    let sanitized = sanitize_for_logging(input);
    
    assert!(!sanitized.contains("123"), "CVV should be masked");
    assert!(sanitized.contains("[CVV_MASKED]"), "Should contain CVV mask");
}

#[test]
fn test_sanitize_password_in_logs() {
    let input = "password: secret123";
    let sanitized = sanitize_for_logging(input);
    
    assert!(!sanitized.contains("secret123"), "Password should be redacted");
    assert!(sanitized.contains("[REDACTED]"), "Should contain redacted marker");
}

#[test]
fn test_sanitize_secret_in_logs() {
    let input = "api_key: sk_test_123456789";
    let sanitized = sanitize_for_logging(input);
    
    assert!(!sanitized.contains("sk_test_123456789"), "Secret should be redacted");
}

#[test]
fn test_sanitize_multiple_sensitive_data() {
    let input = "Card 4111111111111111 cvv: 123 password: abc123";
    let sanitized = sanitize_for_logging(input);
    
    assert!(!sanitized.contains("4111111111111111"));
    assert!(!sanitized.contains("123"));
    assert!(!sanitized.contains("abc123"));
}

#[test]
fn test_sanitize_preserves_normal_text() {
    let input = "Transaction completed successfully for order ORD-12345";
    let sanitized = sanitize_for_logging(input);
    
    assert_eq!(sanitized, input, "Normal text should not be modified");
}

// ============================================================================
// Audit Entry Tests
// ============================================================================

#[test]
fn test_pci_audit_entry_creation() {
    let entry = PciAuditEntry {
        timestamp: chrono::Utc::now(),
        action: PciAuditAction::Read,
        actor: "payment-service".into(),
        data_type: "card_number".into(),
        pan_accessed: true,
        masked_pan: Some("411111XXXXXX1111".into()),
        success: true,
        details: None,
    };
    
    assert_eq!(entry.actor, "payment-service");
    assert!(entry.pan_accessed);
    assert!(entry.success);
}

#[test]
fn test_pci_audit_action_variants() {
    let actions = vec![
        PciAuditAction::Read,
        PciAuditAction::Write,
        PciAuditAction::Delete,
        PciAuditAction::Encrypt,
        PciAuditAction::Decrypt,
        PciAuditAction::KeyRotation,
        PciAuditAction::AccessDenied,
    ];
    
    assert_eq!(actions.len(), 7);
}

#[test]
fn test_pci_audit_entry_serialization() {
    let entry = PciAuditEntry {
        timestamp: chrono::Utc::now(),
        action: PciAuditAction::Write,
        actor: "test".into(),
        data_type: "test".into(),
        pan_accessed: false,
        masked_pan: None,
        success: true,
        details: Some("test details".into()),
    };
    
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: PciAuditEntry = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.actor, "test");
    assert_eq!(deserialized.details, Some("test details".into()));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_pci_error_display() {
    let errors = vec![
        PciError::CvvStorageAttempted("cvv".into()),
        PciError::InvalidPan("too short".into()),
        PciError::KeyNotAvailable("key not found".into()),
        PciError::AccessDenied("insufficient permissions".into()),
        PciError::AuditFailure("write failed".into()),
    ];
    
    for error in errors {
        let display = format!("{}", error);
        assert!(!display.is_empty());
    }
}

#[test]
fn test_pci_error_is_std_error() {
    let error = PciError::CvvStorageAttempted("test".into());
    let _: &dyn std::error::Error = &error;
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_pan_with_leading_zeros() {
    // Some cards may have leading zeros (though rare)
    let result = detect_and_mask_pan("0000000000000001");
    
    // Should still detect as valid format
    assert!(result.found);
}

#[test]
fn test_pan_all_same_digits() {
    let result = detect_and_mask_pan("1111111111111111");
    
    assert!(result.found);
    assert_eq!(result.last_four, Some("1111".to_string()));
}

#[test]
fn test_empty_cvv_field_name() {
    let mut data = serde_json::Map::new();
    data.insert("".to_string(), serde_json::json!("123"));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_ok(), "Empty field name should not trigger error");
}

#[test]
fn test_nested_cvv_in_json() {
    // The function only checks top-level fields
    let mut data = serde_json::Map::new();
    let mut card = serde_json::Map::new();
    card.insert("cvv".to_string(), serde_json::json!("123"));
    data.insert("card".to_string(), serde_json::json!(card));
    
    let result = enforce_no_cvv_storage(&data);
    assert!(result.is_ok(), "Nested CVV should not trigger error (top-level only)");
}

#[test]
fn test_sanitize_empty_string() {
    let sanitized = sanitize_for_logging("");
    assert_eq!(sanitized, "");
}

#[test]
fn test_sanitize_long_text() {
    let input = "A".repeat(10000);
    let sanitized = sanitize_for_logging(&input);
    assert_eq!(sanitized.len(), 10000);
}
