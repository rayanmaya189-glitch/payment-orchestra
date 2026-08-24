//! Platform security utilities for PCI DSS compliance and disaster recovery.
//!
//! This crate provides:
//! - PCI DSS compliance utilities (PAN detection, masking, CVV enforcement)
//! - Disaster recovery and backup/restore procedures
//! - Audit logging for sensitive data access

pub mod pci;
pub mod disaster_recovery;

// Re-exports for convenience
pub use pci::{
    detect_and_mask_pan, enforce_no_cvv_storage, sanitize_for_logging,
    PanDetection, PciAuditEntry, PciAuditAction, PciError,
};
pub use disaster_recovery::{
    DisasterRecoveryManager, BackupConfig, RestoreConfig,
    DrMonitoringConfig, DrCompliance, DrError,
    StorageBackend, BackupType, BackupState, BackupStatus,
};
