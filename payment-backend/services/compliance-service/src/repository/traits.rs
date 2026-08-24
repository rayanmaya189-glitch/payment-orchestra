//! Repository trait for compliance-service.

use uuid::Uuid;

use crate::domain::{KybCase, AmlAlert, ComplianceError, RecentTransaction};

#[allow(dead_code)]
#[async_trait::async_trait]
pub trait ComplianceRepository: Send + Sync {
    // KYB operations
    async fn load_kyb_case(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError>;
    async fn save_kyb_case(&self, kase: &KybCase) -> Result<(), ComplianceError>;
    async fn list_pending_kyb_cases(&self) -> Result<Vec<KybCase>, ComplianceError>;
    async fn find_kyb_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, ComplianceError>;

    // AML operations
    async fn load_aml_alert(&self, id: Uuid) -> Result<Option<AmlAlert>, ComplianceError>;
    async fn save_aml_alert(&self, alert: &AmlAlert) -> Result<(), ComplianceError>;
    async fn list_aml_alerts(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<AmlAlert>, ComplianceError>;
    async fn find_alerts_by_transaction(&self, transaction_id: Uuid) -> Result<Vec<AmlAlert>, ComplianceError>;

    // Transaction history for AML scanning
    async fn get_recent_transactions(&self, operator_id: Uuid, window_minutes: u32) -> Result<Vec<RecentTransaction>, ComplianceError>;
    async fn get_recent_by_method(&self, payment_method_id: &str, window_seconds: u32) -> Result<Vec<RecentTransaction>, ComplianceError>;
    async fn get_average_amount(&self, operator_id: Uuid, min_sample_size: u32) -> Result<f64, ComplianceError>;
}
