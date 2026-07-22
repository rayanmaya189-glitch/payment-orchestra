//! Query handlers for invoice-service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[derive(Debug, Clone)]
pub struct GetInvoiceQuery {
    pub invoice_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct FindInvoiceByOrderQuery {
    pub operator_id: Uuid,
    pub order_reference: String,
}

#[derive(Debug, Clone)]
pub struct FindOverdueInvoicesQuery {
    pub operator_id: Uuid,
}

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_invoice(&self, query: GetInvoiceQuery) -> Result<Option<Invoice>, InvoiceError>;
    async fn find_by_order(&self, query: FindInvoiceByOrderQuery) -> Result<Option<Invoice>, InvoiceError>;
    async fn find_overdue(&self, query: FindOverdueInvoicesQuery) -> Result<Vec<Invoice>, InvoiceError>;
}

pub struct InvoiceQueryHandler<R: InvoiceRepository> {
    repo: R,
}

impl<R: InvoiceRepository> InvoiceQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: InvoiceRepository + Send + Sync> QueryHandler for InvoiceQueryHandler<R> {
    async fn get_invoice(&self, query: GetInvoiceQuery) -> Result<Option<Invoice>, InvoiceError> {
        self.repo.load_invoice(query.invoice_id).await
    }

    async fn find_by_order(&self, query: FindInvoiceByOrderQuery) -> Result<Option<Invoice>, InvoiceError> {
        self.repo.find_by_order_reference(query.operator_id, &query.order_reference).await
    }

    async fn find_overdue(&self, query: FindOverdueInvoicesQuery) -> Result<Vec<Invoice>, InvoiceError> {
        self.repo.find_overdue(query.operator_id).await
    }
}
