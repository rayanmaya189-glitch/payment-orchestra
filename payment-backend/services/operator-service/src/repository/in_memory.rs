//! In-memory repository implementation for Operator aggregate.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{Operator, OperatorError};
use crate::repository::traits::OperatorRepository;

/// In-memory repository for testing and development
#[derive(Clone)]
pub struct InMemoryOperatorRepository {
    pub(super) operators: Arc<RwLock<HashMap<Uuid, Operator>>>,
    pub(super) trade_license_index: Arc<RwLock<HashMap<String, Uuid>>>,
    pub(super) email_index: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl InMemoryOperatorRepository {
    pub fn new() -> Self {
        Self {
            operators: Arc::new(RwLock::new(HashMap::new())),
            trade_license_index: Arc::new(RwLock::new(HashMap::new())),
            email_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryOperatorRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl OperatorRepository for InMemoryOperatorRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Operator>, OperatorError> {
        let ops = self.operators.read().await;
        Ok(ops.get(&id).cloned())
    }

    async fn save(&self, operator: &mut Operator) -> Result<(), OperatorError> {
        let mut ops = self.operators.write().await;
        let mut tli = self.trade_license_index.write().await;
        let mut emi = self.email_index.write().await;

        if let Some(existing) = ops.get(&operator.id) {
            tli.remove(&existing.trade_license_no);
            emi.remove(&existing.email);
        }

        tli.insert(operator.trade_license_no.clone(), operator.id);
        emi.insert(operator.email.clone(), operator.id);

        ops.insert(operator.id, operator.clone());
        Ok(())
    }

    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, OperatorError> {
        let tli = self.trade_license_index.read().await;
        if let Some(id) = tli.get(license) {
            self.load(*id).await
        } else {
            Ok(None)
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, OperatorError> {
        let emi = self.email_index.read().await;
        if let Some(id) = emi.get(email) {
            self.load(*id).await
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    async fn find_by_subdomain(&self, _subdomain: &str) -> Result<Option<Operator>, OperatorError> {
        Ok(None)
    }

    async fn list_by_status(&self, status: Option<&str>) -> Result<Vec<Operator>, OperatorError> {
        let ops = self.operators.read().await;
        let mut result: Vec<Operator> = match status {
            Some(s) => ops.values().filter(|o| o.status.as_str() == s).cloned().collect(),
            None => ops.values().cloned().collect(),
        };
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Operator;

    #[tokio::test]
    async fn test_save_and_load() {
        let repo = InMemoryOperatorRepository::new();
        let mut op = Operator::new(
            Uuid::now_v7(),
            "Test Corp".into(),
            "TEST-123".into(),
            "AE".into(),
            "test@test.com".into(),
            "testcorp".into(),
        );

        repo.save(&mut op).await.unwrap();
        let loaded = repo.load(op.id).await.unwrap().unwrap();
        assert_eq!(loaded.legal_name, "Test Corp");
    }

    #[tokio::test]
    async fn test_find_by_trade_license() {
        let repo = InMemoryOperatorRepository::new();
        let mut op = Operator::new(
            Uuid::now_v7(),
            "Test Corp".into(),
            "CN-99999".into(),
            "AE".into(),
            "test@test.com".into(),
            "testcorp".into(),
        );

        repo.save(&mut op).await.unwrap();
        let found = repo.find_by_trade_license("CN-99999").await.unwrap().unwrap();
        assert_eq!(found.id, op.id);
    }

    #[tokio::test]
    async fn test_find_by_email() {
        let repo = InMemoryOperatorRepository::new();
        let mut op = Operator::new(
            Uuid::now_v7(),
            "Test Corp".into(),
            "CN-88888".into(),
            "AE".into(),
            "unique@test.com".into(),
            "testcorp".into(),
        );

        repo.save(&mut op).await.unwrap();
        let found = repo.find_by_email("unique@test.com").await.unwrap().unwrap();
        assert_eq!(found.id, op.id);
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let repo = InMemoryOperatorRepository::new();
        let result = repo.load(Uuid::now_v7()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = InMemoryOperatorRepository::new();

        let mut op1 = Operator::new(Uuid::now_v7(), "A".into(), "L1".into(), "AE".into(), "a@a.com".into(), "a".into());
        let mut op2 = Operator::new(Uuid::now_v7(), "B".into(), "L2".into(), "AE".into(), "b@b.com".into(), "b".into());

        repo.save(&mut op1).await.unwrap();
        repo.save(&mut op2).await.unwrap();

        let ops = repo.list_by_status(Some("pending")).await.unwrap();
        assert_eq!(ops.len(), 2);
    }
}
