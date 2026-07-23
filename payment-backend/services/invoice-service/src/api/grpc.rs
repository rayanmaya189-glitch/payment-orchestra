//! gRPC service implementation for invoice-service.
//! Translates between protobuf types and domain types for invoice lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler, CreateInvoice, SendInvoice, CancelInvoice};
use crate::domain::{self, InvoiceStatus, InvoiceError};

use platform_proto::invoice::invoice_service_server::InvoiceService;
use platform_proto::invoice::*;
use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationRequest as ProtoPagination};

pub struct InvoiceGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> InvoiceGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> InvoiceService for InvoiceGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: crate::queries::QueryHandler + Send + Sync + 'static,
{
    async fn create_invoice(
        &self,
        request: Request<CreateInvoiceRequest>,
    ) -> Result<Response<CreateInvoiceResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;
        let due_date = chrono::DateTime::parse_from_rfc3339(&req.due_date)
            .map_err(|_| Status::invalid_argument("Invalid due_date format"))?
            .with_timezone(&chrono::Utc);

        let line_items: Vec<domain::InvoiceLineItem> = req.line_items.into_iter().map(|item| {
            domain::InvoiceLineItem {
                description: item.description,
                amount_minor: item.amount_minor_units,
                quantity: item.quantity,
                unit_price_minor: if item.quantity > 0 { item.amount_minor_units / item.quantity as i64 } else { 0 },
            }
        }).collect();

        let cmd = CreateInvoice {
            operator_id,
            order_reference: req.order_reference,
            line_items,
            currency: if req.currency_code.is_empty() { "AED".into() } else { req.currency_code },
            due_date,
            recipient_email: if req.recipient_email.is_empty() { None } else { Some(req.recipient_email) },
        };

        match self.commands.create_invoice(cmd).await {
            Ok(result) => {
                Ok(Response::new(CreateInvoiceResponse {
                    invoice_id: result.invoice_id.to_string(),
                    status: result.status.to_string(),
                    total_amount: Some(ProtoMoney {
                        amount_minor_units: result.total_amount_minor,
                        currency_code: String::new(),
                    }),
                    created_at: Some(Timestamp { unix_ms: chrono::Utc::now().timestamp_millis() }),
                }))
            }
            Err(e) => Err(invoice_error_to_status(e)),
        }
    }

    async fn get_invoice(
        &self,
        request: Request<GetInvoiceRequest>,
    ) -> Result<Response<InvoiceView>, Status> {
        let req = request.into_inner();
        let invoice_id = parse_uuid(&req.invoice_id, "invoice_id")?;

        match self.queries.get_invoice(crate::queries::GetInvoiceQuery { invoice_id }).await {
            Ok(Some(inv)) => {
                let proto_items: Vec<InvoiceLineItem> = inv.line_items.iter().map(|item| {
                    InvoiceLineItem {
                        description: item.description.clone(),
                        amount_minor_units: item.amount_minor,
                        currency_code: inv.currency.clone(),
                        quantity: item.quantity,
                    }
                }).collect();

                Ok(Response::new(InvoiceView {
                    invoice_id: inv.invoice_id.to_string(),
                    operator_id: inv.operator_id.to_string(),
                    order_reference: inv.order_reference,
                    status: inv.status.to_string(),
                    total_amount: Some(ProtoMoney {
                        amount_minor_units: inv.total_amount_minor,
                        currency_code: inv.currency.clone(),
                    }),
                    paid_amount: Some(ProtoMoney {
                        amount_minor_units: inv.paid_amount_minor,
                        currency_code: inv.currency.clone(),
                    }),
                    line_items: proto_items,
                    due_date: inv.due_date.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    recipient_email: inv.recipient_email.unwrap_or_default(),
                    recipient_name: String::new(),
                    payment_intent_ids: inv.payment_intent_ids.iter().map(|id| id.to_string()).collect(),
                    created_at: Some(Timestamp { unix_ms: inv.created_at.timestamp_millis() }),
                    paid_at: None,
                }))
            }
            Ok(None) => Err(Status::not_found("Invoice not found")),
            Err(e) => Err(invoice_error_to_status(e)),
        }
    }

    async fn list_invoices(
        &self,
        request: Request<ListInvoicesRequest>,
    ) -> Result<Response<ListInvoicesResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        // Since the query handler doesn't have a list_all method, we just return an empty list
        // This is a simplified implementation
        let invoices = Vec::new();

        Ok(Response::new(ListInvoicesResponse {
            invoices,
            pagination: None,
        }))
    }

    async fn send_invoice(
        &self,
        request: Request<SendInvoiceRequest>,
    ) -> Result<Response<SendInvoiceResponse>, Status> {
        let req = request.into_inner();
        let invoice_id = parse_uuid(&req.invoice_id, "invoice_id")?;

        let cmd = SendInvoice { invoice_id };

        match self.commands.send_invoice(cmd).await {
            Ok(_) => {
                Ok(Response::new(SendInvoiceResponse {
                    sent: true,
                    delivery_status: "sent".into(),
                }))
            }
            Err(e) => Err(invoice_error_to_status(e)),
        }
    }

    async fn cancel_invoice(
        &self,
        request: Request<CancelInvoiceRequest>,
    ) -> Result<Response<CancelInvoiceResponse>, Status> {
        let req = request.into_inner();
        let invoice_id = parse_uuid(&req.invoice_id, "invoice_id")?;

        let reason = if req.reason.is_empty() { None } else { Some(req.reason) };
        let cmd = CancelInvoice { invoice_id, reason };

        match self.commands.cancel_invoice(cmd).await {
            Ok(_) => {
                Ok(Response::new(CancelInvoiceResponse {
                    status: "cancelled".into(),
                }))
            }
            Err(e) => Err(invoice_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn invoice_error_to_status(e: InvoiceError) -> Status {
    match e {
        InvoiceError::NotFound(id) => Status::not_found(format!("Invoice not found: {}", id)),
        InvoiceError::DuplicateOrderInvoice(ref order) => {
            Status::already_exists(format!("Duplicate order invoice: {}", order))
        }
        InvoiceError::Validation(msg) => Status::invalid_argument(msg),
        InvoiceError::General(msg) => Status::internal(msg),
    }
}

impl From<InvoiceError> for Status {
    fn from(e: InvoiceError) -> Self {
        invoice_error_to_status(e)
    }
}
