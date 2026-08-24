//! ComplianceRepository trait implementation for PostgresComplianceRepository.
//!
//! Delegates KybCase/AmlAlert operations to the sub-modules and handles
//! transaction history queries via in-memory stores.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::{AmlAlert, ComplianceError, KybCase, RecentTransaction};
use super::PostgresComplianceRepository;
use crate::repository::ComplianceRepository;

#[async_trait]
impl ComplianceRepository for PostgresComplianceRepository {
    // ─── KYB Operations ───────────────────────────────────────────────────

    async fn load_kyb_case(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        self.load_kyb_case_domain(id).await
    }

    async fn save_kyb_case(&self, kase: &KybCase) -> Result<(), ComplianceError> {
        self.save_kyb_case_domain(kase).await
    }

    async fn list_pending_kyb_cases(&self) -> Result<Vec<KybCase>, ComplianceError> {
        self.list_pending_kyb_cases_domain().await
    }

    async fn find_kyb_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        self.find_kyb_by_operator_domain(operator_id).await
    }

    // ─── AML Operations ───────────────────────────────────────────────────

    async fn load_aml_alert(&self, id: Uuid) -> Result<Option<AmlAlert>, ComplianceError> {
        self.load_aml_alert_domain(id).await
    }

    async fn save_aml_alert(&self, alert: &AmlAlert) -> Result<(), ComplianceError> {
        self.save_aml_alert_domain(alert).await
    }

    async fn list_aml_alerts(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<AmlAlert>, ComplianceError> {
        self.list_aml_alerts_domain(operator_id, status_filter).await
    }

    async fn find_alerts_by_transaction(&self, transaction_id: Uuid) -> Result<Vec<AmlAlert>, ComplianceError> {
        self.find_alerts_by_transaction_domain(transaction_id).await
    }

    // ─── Transaction History for AML Scanning (In-Memory) ─────────────────

    async fn get_recent_transactions(&self, operator_id: Uuid, window_minutes: u32) -> Result<Vec<RecentTransaction>, ComplianceError> {
        let cutoff = Utc::now() - chrono::Duration::minutes(window_minutes as i64);
        let all = self.recent_txns.read().await;
        let txns: Vec<RecentTransaction> = all.iter()
            .filter(|t| t.timestamp >= cutoff)
            .cloned()
            .collect();

        // Also filter by operator if we have per-operator data
        let op_txns = self.operator_txns.read().await;
        if let Some(txns_for_op) = op_txns.get(&operator_id) {
            let op_filtered: Vec<RecentTransaction> = txns_for_op.iter()
                .filter(|t| t.timestamp >= cutoff)
                .cloned()
                .collect();
            if !op_filtered.is_empty() {
                return Ok(op_filtered);
            }
        }

        Ok(txns)
    }

    async fn get_recent_by_method(&self, payment_method_id: &str, window_seconds: u32) -> Result<Vec<RecentTransaction>, ComplianceError> {
        let cutoff = Utc::now() - chrono::Duration::seconds(window_seconds as i64);
        let method_txns = self.method_txns.read().await;
        if let Some(txns) = method_txns.get(payment_method_id) {
            Ok(txns.iter()
                .filter(|t| t.timestamp >= cutoff)
                .cloned()
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_average_amount(&self, _operator_id: Uuid, min_sample_size: u32) -> Result<f64, ComplianceError> {
        let all = self.recent_txns.read().await;
        if all.len() < min_sample_size as usize {
            return Ok(0.0);
        }
        let total: i64 = all.iter().map(|t| t.amount_minor_units).sum();
        Ok(total as f64 / all.len() as f64)
    }
}
