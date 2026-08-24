//! Query type definitions for invoice-service.

use uuid::Uuid;

use crate::domain::InvoiceStatus;

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

#[derive(Debug, Clone)]
pub struct ListInvoicesQuery {
    pub operator_id: Uuid,
    pub status_filter: Option<InvoiceStatus>,
}
