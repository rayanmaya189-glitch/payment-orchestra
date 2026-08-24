//! Command processing pipeline for invoice-service.

use tracing::{info, error};

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

pub struct InvoicePipeline<H: CommandHandler, Q: QueryHandler> {
    handler: H,
    query_handler: Q,
}

impl<H: CommandHandler, Q: QueryHandler> InvoicePipeline<H, Q> {
    pub fn new(handler: H, query_handler: Q) -> Self {
        Self { handler, query_handler }
    }
}

impl<H: CommandHandler, Q: QueryHandler> InvoicePipeline<H, Q> {
    pub async fn create_invoice(&self, cmd: CreateInvoice) -> Result<InvoiceResult, InvoiceError> {
        info!(operator_id = %cmd.operator_id, order_ref = %cmd.order_reference, "Creating invoice");
        let result = self.handler.create_invoice(cmd).await;
        if let Err(e) = &result {
            error!(error = %e, "Failed to create invoice");
        }
        result
    }

    pub async fn send_invoice(&self, cmd: SendInvoice) -> Result<InvoiceResult, InvoiceError> {
        info!(invoice_id = %cmd.invoice_id, "Sending invoice");
        self.handler.send_invoice(cmd).await
    }

    pub async fn cancel_invoice(&self, cmd: CancelInvoice) -> Result<InvoiceResult, InvoiceError> {
        info!(invoice_id = %cmd.invoice_id, "Cancelling invoice");
        self.handler.cancel_invoice(cmd).await
    }

    pub async fn link_payment(&self, cmd: LinkPaymentToInvoice) -> Result<InvoiceResult, InvoiceError> {
        info!(invoice_id = %cmd.invoice_id, amount = cmd.amount_minor, "Linking payment to invoice");
        self.handler.link_payment(cmd).await
    }

    pub async fn get_invoice(&self, query: GetInvoiceQuery) -> Result<Option<Invoice>, InvoiceError> {
        self.query_handler.get_invoice(query).await
    }
}
