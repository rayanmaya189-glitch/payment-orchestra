//! Command handler trait and implementation for invoice-service.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use crate::events::*;
use crate::repository::*;
use super::types::*;

// ─── Command Handler Trait ───────────────────────────────────────────────────

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_invoice(&self, cmd: CreateInvoice) -> Result<InvoiceResult, InvoiceError>;
    async fn send_invoice(&self, cmd: SendInvoice) -> Result<InvoiceResult, InvoiceError>;
    async fn cancel_invoice(&self, cmd: CancelInvoice) -> Result<InvoiceResult, InvoiceError>;
    async fn link_payment(&self, cmd: LinkPaymentToInvoice) -> Result<InvoiceResult, InvoiceError>;
    async fn mark_overdue(&self, cmd: MarkInvoiceOverdue) -> Result<InvoiceResult, InvoiceError>;
}

// ─── Handler Implementation ──────────────────────────────────────────────────

pub struct InvoiceCommandHandler<R: InvoiceRepository> {
    repo: R,
}

impl<R: InvoiceRepository> InvoiceCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: InvoiceRepository + Send + Sync> CommandHandler for InvoiceCommandHandler<R> {
    async fn create_invoice(&self, cmd: CreateInvoice) -> Result<InvoiceResult, InvoiceError> {
        if self.repo.find_by_order_reference(cmd.operator_id, &cmd.order_reference).await?.is_some() {
            return Err(InvoiceError::DuplicateOrderInvoice(cmd.order_reference));
        }

        let invoice_id = Uuid::now_v7();
        let mut invoice = Invoice::new(
            invoice_id,
            cmd.operator_id,
            cmd.order_reference.clone(),
            cmd.line_items,
            cmd.currency,
            cmd.due_date,
            cmd.recipient_email.clone(),
        )?;

        let event = InvoiceEvent::InvoiceCreated(InvoiceCreated {
            invoice_id,
            operator_id: cmd.operator_id,
            order_reference: cmd.order_reference,
            total_amount_minor: invoice.total_amount_minor,
            currency: invoice.currency.clone(),
            due_date: invoice.due_date,
            recipient_email: cmd.recipient_email,
            occurred_at: Utc::now(),
        });

        invoice.apply_event(&event);

        self.repo.save_invoice(&mut invoice).await?;

        Ok(InvoiceResult {
            invoice_id,
            status: invoice.status,
            total_amount_minor: invoice.total_amount_minor,
            paid_amount_minor: 0,
            event,
        })
    }

    async fn send_invoice(&self, cmd: SendInvoice) -> Result<InvoiceResult, InvoiceError> {
        let mut invoice = self.repo.load_invoice(cmd.invoice_id).await?
            .ok_or(InvoiceError::NotFound(cmd.invoice_id))?;

        invoice.status.can_transition_to(&InvoiceStatus::Sent)?;

        let event = InvoiceEvent::InvoiceSent(InvoiceSent {
            invoice_id: cmd.invoice_id,
            occurred_at: Utc::now(),
        });

        invoice.apply_event(&event);

        self.repo.save_invoice(&mut invoice).await?;

        Ok(InvoiceResult {
            invoice_id: cmd.invoice_id,
            status: invoice.status,
            total_amount_minor: invoice.total_amount_minor,
            paid_amount_minor: invoice.paid_amount_minor,
            event,
        })
    }

    async fn cancel_invoice(&self, cmd: CancelInvoice) -> Result<InvoiceResult, InvoiceError> {
        let mut invoice = self.repo.load_invoice(cmd.invoice_id).await?
            .ok_or(InvoiceError::NotFound(cmd.invoice_id))?;

        invoice.cancel()?;

        let event = InvoiceEvent::InvoiceCancelled(InvoiceCancelled {
            invoice_id: cmd.invoice_id,
            reason: cmd.reason,
            occurred_at: Utc::now(),
        });

        invoice.apply_event(&event);

        self.repo.save_invoice(&mut invoice).await?;

        Ok(InvoiceResult {
            invoice_id: cmd.invoice_id,
            status: invoice.status,
            total_amount_minor: invoice.total_amount_minor,
            paid_amount_minor: invoice.paid_amount_minor,
            event,
        })
    }

    async fn link_payment(&self, cmd: LinkPaymentToInvoice) -> Result<InvoiceResult, InvoiceError> {
        let mut invoice = self.repo.load_invoice(cmd.invoice_id).await?
            .ok_or(InvoiceError::NotFound(cmd.invoice_id))?;

        invoice.apply_payment(cmd.amount_minor)?;
        invoice.payment_intent_ids.push(cmd.payment_intent_id);

        let event = if invoice.status == InvoiceStatus::Paid {
            InvoiceEvent::InvoicePaid(InvoicePaid {
                invoice_id: cmd.invoice_id,
                payment_intent_id: cmd.payment_intent_id,
                paid_amount_minor: invoice.paid_amount_minor,
                fully_paid: true,
                occurred_at: Utc::now(),
            })
        } else {
            InvoiceEvent::InvoicePartiallyPaid(InvoicePartiallyPaid {
                invoice_id: cmd.invoice_id,
                payment_intent_id: cmd.payment_intent_id,
                paid_amount_minor: cmd.amount_minor,
                remaining_minor: invoice.total_amount_minor - invoice.paid_amount_minor,
                occurred_at: Utc::now(),
            })
        };

        invoice.apply_event(&event);

        self.repo.save_invoice(&mut invoice).await?;

        Ok(InvoiceResult {
            invoice_id: cmd.invoice_id,
            status: invoice.status,
            total_amount_minor: invoice.total_amount_minor,
            paid_amount_minor: invoice.paid_amount_minor,
            event,
        })
    }

    async fn mark_overdue(&self, cmd: MarkInvoiceOverdue) -> Result<InvoiceResult, InvoiceError> {
        let mut invoice = self.repo.load_invoice(cmd.invoice_id).await?
            .ok_or(InvoiceError::NotFound(cmd.invoice_id))?;

        invoice.mark_overdue()?;

        let days_overdue = Utc::now().signed_duration_since(invoice.due_date).num_days() as u32;
        let event = InvoiceEvent::InvoiceOverdue(InvoiceOverdue {
            invoice_id: cmd.invoice_id,
            due_date: invoice.due_date,
            days_overdue,
            occurred_at: Utc::now(),
        });

        invoice.apply_event(&event);

        self.repo.save_invoice(&mut invoice).await?;

        Ok(InvoiceResult {
            invoice_id: cmd.invoice_id,
            status: invoice.status,
            total_amount_minor: invoice.total_amount_minor,
            paid_amount_minor: invoice.paid_amount_minor,
            event,
        })
    }
}
