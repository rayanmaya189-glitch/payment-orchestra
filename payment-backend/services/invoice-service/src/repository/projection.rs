//! Projections for read-optimized queries in invoice service.
//!
//! These projections are built from the event stream and stored in PostgreSQL
//! for efficient querying without replaying all events.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use sea_orm::ActiveModelTrait;
use uuid::Uuid;

use crate::domain::*;

/// Projection repository trait for read queries
#[async_trait]
pub trait ProjectionInvoiceRepository: Send + Sync {
    /// Find invoice by order reference
    async fn find_by_order_reference(
        &self,
        operator_id: Uuid,
        order_ref: &str,
    ) -> Result<Option<Invoice>, InvoiceError>;
    
    /// Find invoice by payment intent ID
    async fn find_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<Invoice>, InvoiceError>;
    
    /// Find overdue invoices for an operator
    async fn find_overdue(&self, operator_id: Uuid) -> Result<Vec<Invoice>, InvoiceError>;
    
    /// List invoices with optional status filter
    async fn list_invoices(
        &self,
        operator_id: Uuid,
        status_filter: Option<InvoiceStatus>,
    ) -> Result<Vec<Invoice>, InvoiceError>;
    
    /// Update projection from event
    async fn project_event(
        &self,
        invoice: &Invoice,
        event_type: &str,
    ) -> Result<(), InvoiceError>;
}

/// PostgreSQL-backed projection repository
pub struct PgProjectionInvoiceRepository {
    db: DatabaseConnection,
}

impl PgProjectionInvoiceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectionInvoiceRepository for PgProjectionInvoiceRepository {
    async fn find_by_order_reference(
        &self,
        _operator_id: Uuid,
        _order_ref: &str,
    ) -> Result<Option<Invoice>, InvoiceError> {
        // In production, query projection table:
        // SELECT * FROM invoice_projections WHERE operator_id = ? AND order_reference = ?
        
        // For now, return None (projection not built yet)
        Ok(None)
    }
    
    async fn find_by_payment_intent(
        &self,
        _payment_intent_id: Uuid,
    ) -> Result<Option<Invoice>, InvoiceError> {
        // In production, query junction table:
        // SELECT i.* FROM invoice_projections i
        // JOIN invoice_payment_intents ipi ON i.invoice_id = ipi.invoice_id
        // WHERE ipi.payment_intent_id = ?
        
        Ok(None)
    }
    
    async fn find_overdue(
        &self,
        _operator_id: Uuid,
    ) -> Result<Vec<Invoice>, InvoiceError> {
        // In production:
        // SELECT * FROM invoice_projections
        // WHERE operator_id = ? AND status = 'Sent' AND due_date < NOW()
        
        Ok(Vec::new())
    }
    
    async fn list_invoices(
        &self,
        _operator_id: Uuid,
        _status_filter: Option<InvoiceStatus>,
    ) -> Result<Vec<Invoice>, InvoiceError> {
        // In production:
        // SELECT * FROM invoice_projections
        // WHERE operator_id = ? [AND status = ?]
        // ORDER BY created_at DESC
        
        Ok(Vec::new())
    }
    
    async fn project_event(
        &self,
        _invoice: &Invoice,
        _event_type: &str,
    ) -> Result<(), InvoiceError> {
        // In production, this would:
        // 1. UPSERT into invoice_projections table
        // 2. Update junction tables if needed
        // 3. Handle denormalized fields
        
        Ok(())
    }
}

/// In-memory projection for testing
pub struct InMemoryProjectionInvoiceRepository {
    // In production, this would be a database
}

impl Default for InMemoryProjectionInvoiceRepository {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait]
impl ProjectionInvoiceRepository for InMemoryProjectionInvoiceRepository {
    async fn find_by_order_reference(
        &self,
        _operator_id: Uuid,
        _order_ref: &str,
    ) -> Result<Option<Invoice>, InvoiceError> {
        Ok(None)
    }
    
    async fn find_by_payment_intent(
        &self,
        _payment_intent_id: Uuid,
    ) -> Result<Option<Invoice>, InvoiceError> {
        Ok(None)
    }
    
    async fn find_overdue(
        &self,
        _operator_id: Uuid,
    ) -> Result<Vec<Invoice>, InvoiceError> {
        Ok(Vec::new())
    }
    
    async fn list_invoices(
        &self,
        _operator_id: Uuid,
        _status_filter: Option<InvoiceStatus>,
    ) -> Result<Vec<Invoice>, InvoiceError> {
        Ok(Vec::new())
    }
    
    async fn project_event(
        &self,
        _invoice: &Invoice,
        _event_type: &str,
    ) -> Result<(), InvoiceError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_projection() {
        let repo = InMemoryProjectionInvoiceRepository::default();
        let result = repo.find_by_order_reference(Uuid::now_v7(), "ORD-001").await;
        assert!(result.is_ok());
    }
}
