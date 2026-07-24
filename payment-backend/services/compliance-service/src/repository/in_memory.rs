//! In-memory repository for compliance-service.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{KybCase, AmlAlert, ComplianceError, RecentTransaction};
use crate::repository::traits::ComplianceRepository;

#[derive(Clone)]
pub struct InMemoryComplianceRepository {
    pub(super) kyb_cases: Arc<RwLock<HashMap<Uuid, KybCase>>>,
    pub(super) operator_index: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    pub(super) aml_alerts: Arc<RwLock<HashMap<Uuid, AmlAlert>>>,
    pub(super) recent_txns: Arc<RwLock<Vec<RecentTransaction>>>,
}

impl InMemoryComplianceRepository {
    pub fn new() -> Self {
        Self {
            kyb_cases: Arc::new(RwLock::new(HashMap::new())),
            operator_index: Arc::new(RwLock::new(HashMap::new())),
            aml_alerts: Arc::new(RwLock::new(HashMap::new())),
            recent_txns: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryComplianceRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to add recent transactions (for testing)
impl InMemoryComplianceRepository {
    #[allow(dead_code)]
    pub async fn add_transactions(&self, txns: Vec<RecentTransaction>) {
        let mut map = self.recent_txns.write().await;
        map.extend(txns);
    }
}

#[async_trait::async_trait]
impl ComplianceRepository for InMemoryComplianceRepository {
    async fn load_kyb_case(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        let map = self.kyb_cases.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save_kyb_case(&self, kase: &KybCase) -> Result<(), ComplianceError> {
        let mut map = self.kyb_cases.write().await;
        let mut idx = self.operator_index.write().await;
        idx.insert(kase.operator_id, kase.kyb_case_id);
        map.insert(kase.kyb_case_id, kase.clone());
        Ok(())
    }

    async fn list_pending_kyb_cases(&self) -> Result<Vec<KybCase>, ComplianceError> {
        let map = self.kyb_cases.read().await;
        let mut cases: Vec<KybCase> = map.values()
            .filter(|c| !c.status.is_terminal())
            .cloned()
            .collect();
        cases.sort_by_key(|a| a.submitted_at);
        Ok(cases)
    }

    async fn find_kyb_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        let idx = self.operator_index.read().await;
        if let Some(case_id) = idx.get(&operator_id) {
            self.load_kyb_case(*case_id).await
        } else {
            Ok(None)
        }
    }

    async fn load_aml_alert(&self, id: Uuid) -> Result<Option<AmlAlert>, ComplianceError> {
        let map = self.aml_alerts.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save_aml_alert(&self, alert: &AmlAlert) -> Result<(), ComplianceError> {
        let mut map = self.aml_alerts.write().await;
        map.insert(alert.alert_id, alert.clone());
        Ok(())
    }

    async fn list_aml_alerts(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<AmlAlert>, ComplianceError> {
        let map = self.aml_alerts.read().await;
        let alerts: Vec<AmlAlert> = map.values()
            .filter(|a| a.operator_id == operator_id)
            .filter(|a| status_filter.is_none_or(|s| a.status.as_str() == s))
            .cloned()
            .collect();
        Ok(alerts)
    }

    async fn find_alerts_by_transaction(&self, transaction_id: Uuid) -> Result<Vec<AmlAlert>, ComplianceError> {
        let map = self.aml_alerts.read().await;
        let alerts: Vec<AmlAlert> = map.values()
            .filter(|a| a.transaction_id == transaction_id)
            .cloned()
            .collect();
        Ok(alerts)
    }

    async fn get_recent_transactions(&self, _operator_id: Uuid, _window_minutes: u32) -> Result<Vec<RecentTransaction>, ComplianceError> {
        let txns = self.recent_txns.read().await;
        Ok(txns.clone())
    }

    async fn get_recent_by_method(&self, _payment_method_id: &str, _window_seconds: u32) -> Result<Vec<RecentTransaction>, ComplianceError> {
        let txns = self.recent_txns.read().await;
        Ok(txns.clone())
    }

    async fn get_average_amount(&self, _operator_id: Uuid, _min_sample_size: u32) -> Result<f64, ComplianceError> {
        let txns = self.recent_txns.read().await;
        if txns.is_empty() {
            return Ok(0.0);
        }
        let total: i64 = txns.iter().map(|t| t.amount_minor_units).sum();
        Ok(total as f64 / txns.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::KybCase;

    #[tokio::test]
    async fn test_save_and_load_kyb_case() {
        let repo = InMemoryComplianceRepository::new();
        let kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![Uuid::now_v7()]);
        repo.save_kyb_case(&kase).await.unwrap();
        let loaded = repo.load_kyb_case(kase.kyb_case_id).await.unwrap().unwrap();
        assert_eq!(loaded.kyb_case_id, kase.kyb_case_id);
    }

    #[tokio::test]
    async fn test_list_pending_cases() {
        let repo = InMemoryComplianceRepository::new();
        let kase = KybCase::new(Uuid::now_v7(), Uuid::now_v7(), vec![]);
        repo.save_kyb_case(&kase).await.unwrap();
        let pending = repo.list_pending_kyb_cases().await.unwrap();
        assert_eq!(pending.len(), 1);
    }
}
