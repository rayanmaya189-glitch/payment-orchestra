//! Domain model for BC-03 Merchant Compliance.
//! Owns KybCase aggregate, AmlAlert entity, AML rule engine, and SAR reports.
//!
//! Each domain concept has its own file within this module.

// Sub-modules — one file per concept
pub mod aml_alert;
pub mod aml_monitor;
pub mod error;
pub mod kyb_case;
pub mod sar_report;

// Re-export all types for convenience
pub use aml_alert::{AlertSeverity, AlertStatus, AmlAlert};
#[allow(unused_imports)]
pub use aml_alert::AmlAlertType;
pub use aml_monitor::{AmlMonitor, RecentTransaction};
pub use error::ComplianceError;
pub use kyb_case::KybCase;
#[allow(unused_imports)]
pub use kyb_case::KybStatus;


// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_kyb_case_creation() {
        let kase = KybCase::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            vec![Uuid::now_v7()],
        );
        assert_eq!(kase.status, KybStatus::Submitted);
        assert!(!kase.status.is_terminal());
    }

    #[test]
    fn test_kyb_approve() {
        let mut kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![]);
        assert!(kase.approve().is_ok());
        assert_eq!(kase.status, KybStatus::Approved);
        assert!(kase.status.is_terminal());
    }

    #[test]
    fn test_kyb_reject() {
        let mut kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![]);
        assert!(kase.reject("Invalid documents".into()).is_ok());
        assert_eq!(kase.status, KybStatus::Rejected);
        assert_eq!(kase.rejection_reason.unwrap(), "Invalid documents");
    }

    #[test]
    fn test_kyb_double_approve_fails() {
        let mut kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![]);
        kase.approve().unwrap();
        assert!(kase.approve().is_err());
    }

    #[test]
    fn test_aml_monitor_velocity() {
        let monitor = AmlMonitor::new();
        let operator_id = Uuid::now_v7();
        let recent: Vec<RecentTransaction> = (0..25)
            .map(|i| RecentTransaction {
                transaction_id: Uuid::now_v7(),
                amount_minor_units: 1000 + i * 100,
                payment_method_id: None,
                timestamp: Utc::now(),
            })
            .collect();

        let alerts = monitor.scan(
            Uuid::now_v7(),
            operator_id,
            5000,
            &recent,
            &[],
            0.0,
        );

        let velocity_alerts: Vec<_> = alerts.iter().filter(|a| a.alert_type == AmlAlertType::Velocity).collect();
        assert!(!velocity_alerts.is_empty());
    }

    #[test]
    fn test_aml_monitor_no_alert_for_normal_traffic() {
        let monitor = AmlMonitor::new();
        let recent: Vec<RecentTransaction> = (0..3)
            .map(|i| RecentTransaction {
                transaction_id: Uuid::now_v7(),
                amount_minor_units: 1000 + i * 100,
                payment_method_id: None,
                timestamp: Utc::now(),
            })
            .collect();

        let alerts = monitor.scan(
            Uuid::now_v7(),
            Uuid::now_v7(),
            5000,
            &recent,
            &[],
            0.0,
        );

        assert!(alerts.is_empty());
    }

    #[test]
    fn test_kyb_status_transitions() {
        assert!(!KybStatus::Submitted.is_terminal());
        assert!(!KybStatus::UnderReview.is_terminal());
        assert!(KybStatus::Approved.is_terminal());
        assert!(KybStatus::Rejected.is_terminal());
    }
}
