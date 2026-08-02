//! Projections for read-optimized queries in reconciliation service.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::*;

/// Projection repository trait for read queries
#[async_trait]
pub trait ProjectionReconciliationRepository: Send + Sync {
    /// Find reconciliation by date range
    async fn find_by_date_range(
        &self,
        operator_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Reconciliation>, ReconciliationError>;
    
    /// Find pending reconciliation items
    async fn find_pending(&self, operator_id: Uuid) -> Result<Vec<ReconciliationItem>, ReconciliationError>;
    
    /// Find unmatched settlements
    async fn find_unmatched_settlements(
        &self,
        operator_id: Uuid,
        connector_id: &str,
    ) -> Result<Vec<SettlementRecord>, ReconciliationError>;
    
    /// Get reconciliation summary
    async fn get_summary(
        &self,
        operator_id: Uuid,
        date: DateTime<Utc>,
    ) -> Result<ReconciliationSummary, ReconciliationError>;
    
    /// Update projection from event
    async fn project_event(&self, reconciliation: &Reconciliation, event_type: &str) -> Result<(), ReconciliationError>;
}

/// Reconciliation summary
#[derive(Debug, Clone)]
pub struct ReconciliationSummary {
    pub date: DateTime<Utc>,
    pub total_expected: i64,
    pub total_matched: i64,
    pub total_unmatched: i64,
    pub match_rate: f64,
}

/// In-memory projection for testing
pub struct InMemoryProjectionReconciliationRepository;

impl Default for InMemoryProjectionReconciliationRepository {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl ProjectionReconciliationRepository for InMemoryProjectionReconciliationRepository {
    async fn find_by_date_range(
        &self,
        _operator_id: Uuid,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Reconciliation>, ReconciliationError> {
        Ok(Vec::new())
    }
    
    async fn find_pending(&self, _operator_id: Uuid) -> Result<Vec<ReconciliationItem>, ReconciliationError> {
        Ok(Vec::new())
    }
    
    async fn find_unmatched_settlements(
        &self,
        _operator_id: Uuid,
        _connector_id: &str,
    ) -> Result<Vec<SettlementRecord>, ReconciliationError> {
        Ok(Vec::new())
    }
    
    async fn get_summary(
        &self,
        _operator_id: Uuid,
        _date: DateTime<Utc>,
    ) -> Result<ReconciliationSummary, ReconciliationError> {
        Ok(ReconciliationSummary {
            date: Utc::now(),
            total_expected: 0,
            total_matched: 0,
            total_unmatched: 0,
            match_rate: 100.0,
        })
    }
    
    async fn project_event(&self, _reconciliation: &Reconciliation, _event_type: &str) -> Result<(), ReconciliationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_projection_repository() {
        let repo = InMemoryProjectionReconciliationRepository::default();
        let result = repo.find_pending(Uuid::now_v7()).await;
        assert!(result.is_ok());
    }
}
