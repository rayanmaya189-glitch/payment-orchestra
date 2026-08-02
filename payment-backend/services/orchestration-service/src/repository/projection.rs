//! Projections for read-optimized queries in orchestration service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

/// Projection repository trait for read queries
#[async_trait]
pub trait ProjectionOrchestrationRepository: Send + Sync {
    /// Find payment intent by order reference
    async fn find_by_order_reference(
        &self,
        operator_id: Uuid,
        order_ref: &str,
    ) -> Result<Option<PaymentIntent>, OrchestrationError>;
    
    /// Find payment intents by status
    async fn find_by_status(
        &self,
        operator_id: Uuid,
        status: PaymentIntentStatus,
    ) -> Result<Vec<PaymentIntent>, OrchestrationError>;
    
    /// Find pending captures
    async fn find_pending_captures(&self) -> Result<Vec<PaymentIntent>, OrchestrationError>;
    
    /// List payment intents with filters
    async fn list_intents(
        &self,
        operator_id: Uuid,
        status_filter: Option<PaymentIntentStatus>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<PaymentIntent>, OrchestrationError>;
    
    /// Update projection from event
    async fn project_event(&self, intent: &PaymentIntent, event_type: &str) -> Result<(), OrchestrationError>;
}

/// In-memory projection for testing
pub struct InMemoryProjectionOrchestrationRepository;

impl Default for InMemoryProjectionOrchestrationRepository {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl ProjectionOrchestrationRepository for InMemoryProjectionOrchestrationRepository {
    async fn find_by_order_reference(
        &self,
        _operator_id: Uuid,
        _order_ref: &str,
    ) -> Result<Option<PaymentIntent>, OrchestrationError> {
        Ok(None)
    }
    
    async fn find_by_status(
        &self,
        _operator_id: Uuid,
        _status: PaymentIntentStatus,
    ) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        Ok(Vec::new())
    }
    
    async fn find_pending_captures(&self) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        Ok(Vec::new())
    }
    
    async fn list_intents(
        &self,
        _operator_id: Uuid,
        _status_filter: Option<PaymentIntentStatus>,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        Ok(Vec::new())
    }
    
    async fn project_event(&self, _intent: &PaymentIntent, _event_type: &str) -> Result<(), OrchestrationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_projection_repository() {
        let repo = InMemoryProjectionOrchestrationRepository::default();
        let result = repo.find_pending_captures().await;
        assert!(result.is_ok());
    }
}
