//! Invoice repository trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn load_invoice(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError>;
    async fn save_invoice(&self, invoice: &Invoice) -> Result<(), InvoiceError>;
    async fn find_by_order_reference(&self, operator_id: Uuid, order_ref: &str) -> Result<Option<Invoice>, InvoiceError>;
    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<Invoice>, InvoiceError>;
    async fn find_overdue(&self, operator_id: Uuid) -> Result<Vec<Invoice>, InvoiceError>;
    async fn list_invoices(&self, operator_id: Uuid, status_filter: Option<InvoiceStatus>) -> Result<Vec<Invoice>, InvoiceError>;
}
