//! In-memory invoice repository.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::InvoiceRepository;

#[derive(Clone)]
pub struct InMemoryInvoiceRepository {
    pub(super) invoices: Arc<RwLock<HashMap<Uuid, Invoice>>>,
    pub(super) order_refs: Arc<RwLock<HashMap<(Uuid, String), Uuid>>>,
    pub(super) payment_intent_refs: Arc<RwLock<HashMap<Uuid, Uuid>>>,
}

impl Default for InMemoryInvoiceRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryInvoiceRepository {
    pub fn new() -> Self {
        Self {
            invoices: Arc::new(RwLock::new(HashMap::new())),
            order_refs: Arc::new(RwLock::new(HashMap::new())),
            payment_intent_refs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl InvoiceRepository for InMemoryInvoiceRepository {
    async fn load_invoice(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError> {
        let store = self.invoices.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn save_invoice(&self, invoice: &Invoice) -> Result<(), InvoiceError> {
        let id = invoice.invoice_id;
        let op_id = invoice.operator_id;
        let order_ref = invoice.order_reference.clone();
        self.invoices.write().await.insert(id, invoice.clone());
        self.order_refs.write().await.insert((op_id, order_ref), id);
        for pi_id in &invoice.payment_intent_ids {
            self.payment_intent_refs.write().await.insert(*pi_id, id);
        }
        Ok(())
    }

    async fn find_by_order_reference(&self, operator_id: Uuid, order_ref: &str) -> Result<Option<Invoice>, InvoiceError> {
        let refs = self.order_refs.read().await;
        if let Some(invoice_id) = refs.get(&(operator_id, order_ref.to_string())) {
            let store = self.invoices.read().await;
            return Ok(store.get(invoice_id).cloned());
        }
        Ok(None)
    }

    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<Invoice>, InvoiceError> {
        let refs = self.payment_intent_refs.read().await;
        if let Some(invoice_id) = refs.get(&payment_intent_id) {
            let store = self.invoices.read().await;
            return Ok(store.get(invoice_id).cloned());
        }
        Ok(None)
    }

    async fn find_overdue(&self, operator_id: Uuid) -> Result<Vec<Invoice>, InvoiceError> {
        let store = self.invoices.read().await;
        let now = Utc::now();
        Ok(store.values()
            .filter(|inv| {
                inv.operator_id == operator_id
                    && matches!(inv.status, InvoiceStatus::Sent)
                    && inv.due_date < now
            })
            .cloned()
            .collect())
    }

    async fn list_invoices(&self, operator_id: Uuid, status_filter: Option<InvoiceStatus>) -> Result<Vec<Invoice>, InvoiceError> {
        let store = self.invoices.read().await;
        Ok(store.values()
            .filter(|inv| {
                if inv.operator_id != operator_id {
                    return false;
                }
                if let Some(ref filter) = status_filter {
                    if &inv.status != filter {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect())
    }
}
