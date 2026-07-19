use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;

use crate::domain::aggregates::Invoice;
use crate::domain::value_objects::InvoiceLineItem;
use crate::infrastructure::repository::InvoiceRepository;
use platform_error::{PlatformError, ConflictError};

#[async_trait]
pub trait InvoiceService: Send + Sync {
    async fn create_invoice(&self, cmd: CreateInvoiceCommand) -> Result<InvoiceResponse, PlatformError>;
    async fn send_invoice(&self, cmd: SendInvoiceCommand) -> Result<(), PlatformError>;
    async fn cancel_invoice(&self, cmd: CancelInvoiceCommand) -> Result<(), PlatformError>;
    async fn record_payment(&self, cmd: RecordPaymentCommand) -> Result<(), PlatformError>;
    async fn get_invoice(&self, invoice_id: Uuid) -> Result<InvoiceResponse, PlatformError>;
}

pub struct InvoiceServiceImpl {
    repo: Box<dyn InvoiceRepository>,
}

impl InvoiceServiceImpl {
    pub fn new(repo: Box<dyn InvoiceRepository>) -> Self {
        Self { repo }
    }
}

#[derive(Debug, Clone)]
pub struct CreateInvoiceCommand {
    pub operator_id: Uuid,
    pub order_reference: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub due_date: chrono::DateTime<chrono::Utc>,
    pub recipient_email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SendInvoiceCommand {
    pub invoice_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CancelInvoiceCommand {
    pub invoice_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordPaymentCommand {
    pub invoice_id: Uuid,
    pub payment_intent_id: Uuid,
    pub amount_minor_units: i64,
}

#[derive(Debug, Clone)]
pub struct InvoiceResponse {
    pub invoice_id: Uuid,
    pub order_reference: String,
    pub status: String,
    pub total_amount: i64,
    pub paid_amount: i64,
    pub currency: String,
    pub due_date: String,
    pub recipient_email: Option<String>,
}

#[async_trait]
impl InvoiceService for InvoiceServiceImpl {
    async fn create_invoice(&self, cmd: CreateInvoiceCommand) -> Result<InvoiceResponse, PlatformError> {
        // Check duplicate (INV-INV-01)
        if let Some(_) = self.repo.find_by_order_reference(cmd.operator_id, &cmd.order_reference).await? {
            return Err(PlatformError::Conflict(ConflictError::DuplicateOrderInvoice));
        }

        let invoice = Invoice::new(
            cmd.operator_id,
            cmd.order_reference,
            cmd.line_items,
            cmd.due_date,
            cmd.recipient_email,
        );

        self.repo.save(&invoice).await?;

        Ok(invoice_to_response(&invoice))
    }

    async fn send_invoice(&self, cmd: SendInvoiceCommand) -> Result<(), PlatformError> {
        let mut invoice = self.repo
            .load(cmd.invoice_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Invoice".into(),
                id: cmd.invoice_id,
            })?;

        if !invoice.can_send() {
            return Err(PlatformError::Conflict(ConflictError::IdempotencyKeyConflict));
        }

        invoice.status = crate::domain::value_objects::InvoiceStatus::Sent;
        invoice.updated_at = Utc::now();
        self.repo.save(&invoice).await?;

        Ok(())
    }

    async fn cancel_invoice(&self, cmd: CancelInvoiceCommand) -> Result<(), PlatformError> {
        let mut invoice = self.repo
            .load(cmd.invoice_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Invoice".into(),
                id: cmd.invoice_id,
            })?;

        if !invoice.can_cancel() {
            return Err(PlatformError::Conflict(ConflictError::IdempotencyKeyConflict));
        }

        invoice.status = crate::domain::value_objects::InvoiceStatus::Cancelled;
        invoice.updated_at = Utc::now();
        self.repo.save(&invoice).await?;

        Ok(())
    }

    async fn record_payment(&self, cmd: RecordPaymentCommand) -> Result<(), PlatformError> {
        let mut invoice = self.repo
            .load(cmd.invoice_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Invoice".into(),
                id: cmd.invoice_id,
            })?;

        invoice.record_payment(cmd.amount_minor_units);
        if !invoice.payment_intent_ids.contains(&cmd.payment_intent_id) {
            invoice.payment_intent_ids.push(cmd.payment_intent_id);
        }
        self.repo.save(&invoice).await?;

        Ok(())
    }

    async fn get_invoice(&self, invoice_id: Uuid) -> Result<InvoiceResponse, PlatformError> {
        let invoice = self.repo
            .load(invoice_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Invoice".into(),
                id: invoice_id,
            })?;

        Ok(invoice_to_response(&invoice))
    }
}

fn invoice_to_response(i: &Invoice) -> InvoiceResponse {
    InvoiceResponse {
        invoice_id: i.invoice_id,
        order_reference: i.order_reference.clone(),
        status: i.status.as_str().to_string(),
        total_amount: i.total_amount.amount_minor_units,
        paid_amount: i.paid_amount.amount_minor_units,
        currency: i.total_amount.currency.0.clone(),
        due_date: i.due_date.to_rfc3339(),
        recipient_email: i.recipient_email.clone(),
    }
}
