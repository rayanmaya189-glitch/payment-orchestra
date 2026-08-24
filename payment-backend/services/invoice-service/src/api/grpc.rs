//! gRPC service implementation for invoice-service.
//! Translates between protobuf types and domain types for invoice lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{CommandHandler, CreateInvoice, SendInvoice, CancelInvoice};
use crate::domain::{self, InvoiceStatus, InvoiceError};

use platform_proto::invoice::invoice_service_server::InvoiceService;
use platform_proto::invoice::*;
use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationResponse};

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

        let currency = if req.currency_code.is_empty() { "AED".to_string() } else { req.currency_code };
        let cmd = CreateInvoice {
            operator_id,
            order_reference: req.order_reference,
            line_items,
            currency: currency.clone(),
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
                        currency_code: currency,
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

        let status_filter = if req.status_filter.is_empty() {
            None
        } else {
            Some(req.status_filter.parse::<InvoiceStatus>().map_err(|_| {
                Status::invalid_argument(format!("Invalid status filter: {}", req.status_filter))
            })?)
        };

        let query = crate::queries::ListInvoicesQuery {
            operator_id,
            status_filter,
        };

        let page_limit = req.pagination.as_ref().map_or(50, |p| {
            if p.limit > 0 && p.limit <= 100 { p.limit as usize } else { 50 }
        });
        let cursor = req.pagination.as_ref().and_then(|p| {
            if p.cursor.is_empty() { None } else { Some(p.cursor.clone()) }
        });

        match self.queries.list_invoices(query).await {
            Ok(invoices) => {
                let proto_invoices: Vec<InvoiceView> = invoices.into_iter()
                    .skip_while(|inv| {
                        // Simple cursor pagination: skip until we find the cursor
                        if let Some(ref c) = cursor {
                            inv.invoice_id.to_string() != *c
                        } else {
                            false
                        }
                    })
                    .take(page_limit)
                    .map(|inv| {
                        let proto_items: Vec<InvoiceLineItem> = inv.line_items.iter().map(|item| {
                            InvoiceLineItem {
                                description: item.description.clone(),
                                amount_minor_units: item.amount_minor,
                                currency_code: inv.currency.clone(),
                                quantity: item.quantity,
                            }
                        }).collect();

                        InvoiceView {
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
                        }
                    })
                    .collect();

        // Cursor-based pagination: if cursor is provided, skip results
        // until we find the matching invoice, then return items from there.

                let next_cursor = proto_invoices.last().map(|inv| inv.invoice_id.clone());
                let has_more = proto_invoices.len() >= page_limit;

                Ok(Response::new(ListInvoicesResponse {
                    invoices: proto_invoices,
                    pagination: Some(PaginationResponse {
                        next_cursor: next_cursor.unwrap_or_default(),
                        has_more,
                        as_of_unix_ms: chrono::Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(invoice_error_to_status(e)),
        }
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
        InvoiceError::DatabaseError(msg) => Status::internal(format!("Database error: {}", msg)),
        InvoiceError::General(msg) => Status::internal(msg),
    }
}

impl From<InvoiceError> for Status {
    fn from(e: InvoiceError) -> Self {
        invoice_error_to_status(e)
    }
}
