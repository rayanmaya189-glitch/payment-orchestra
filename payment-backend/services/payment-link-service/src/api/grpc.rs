//! gRPC service implementation for payment-link-service (BC-07).
//! Translates between protobuf types and domain types for payment link lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::{self, PaymentLinkError, PaymentLinkStatus};
use crate::queries::QueryHandler;

use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationResponse};
use platform_proto::payment_link::payment_link_service_server::PaymentLinkService;
use platform_proto::payment_link::*;

pub struct PaymentLinkGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> PaymentLinkGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> PaymentLinkService for PaymentLinkGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn create_payment_link(
        &self,
        request: Request<CreatePaymentLinkRequest>,
    ) -> Result<Response<CreatePaymentLinkResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        let amount = req
            .amount
            .ok_or_else(|| Status::invalid_argument("amount is required"))?;

        let expiry_minutes = if req.expiry_minutes.is_empty() {
            1440
        } else {
            req.expiry_minutes.parse::<u32>().map_err(|_| {
                Status::invalid_argument("Invalid expiry_minutes")
            })?
        };

        // Convert expiry_minutes to days (with minimum 1 day)
        let expires_in_days = ((expiry_minutes as f64) / 1440.0).ceil() as u32;

        let cmd = commands::CreatePaymentLinkCommand {
            operator_id,
            amount_minor_units: amount.amount_minor_units,
            currency: amount.currency_code,
            description: if req.description.is_empty() { None } else { Some(req.description) },
            invoice_id: None,
            expires_in_days: Some(expires_in_days.max(1)),
        };

        match self.commands.create_payment_link(cmd).await {
            Ok(link) => {
                Ok(Response::new(CreatePaymentLinkResponse {
                    payment_link_id: link.payment_link_id.to_string(),
                    url: format!("/checkout/{}", link.token),
                    token: link.token.clone(),
                    expires_at: Some(Timestamp {
                        unix_ms: link.expires_at.timestamp_millis(),
                    }),
                    status: link.status.to_string(),
                }))
            }
            Err(e) => Err(payment_link_error_to_status(e)),
        }
    }

    async fn get_payment_link(
        &self,
        request: Request<GetPaymentLinkRequest>,
    ) -> Result<Response<PaymentLinkView>, Status> {
        let req = request.into_inner();
        let payment_link_id = parse_uuid(&req.payment_link_id, "payment_link_id")?;

        match self.queries.get_payment_link(payment_link_id).await {
            Ok(link) => Ok(Response::new(payment_link_to_view(link))),
            Err(e) => Err(payment_link_error_to_status(e)),
        }
    }

    async fn list_payment_links(
        &self,
        request: Request<ListPaymentLinksRequest>,
    ) -> Result<Response<ListPaymentLinksResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        let status_filter = if req.status_filter.is_empty() {
            None
        } else {
            Some(req.status_filter.parse::<PaymentLinkStatus>().map_err(|_| {
                Status::invalid_argument(format!("Invalid status filter: {}", req.status_filter))
            })?)
        };

        let page_limit = req.pagination.as_ref().map_or(50, |p| {
            if p.limit > 0 && p.limit <= 100 { p.limit as usize } else { 50 }
        });
        let cursor = req.pagination.as_ref().and_then(|p| {
            if p.cursor.is_empty() { None } else { Some(p.cursor.clone()) }
        });

        match self.queries.find_by_operator(operator_id).await {
            Ok(all_links) => {
                let filtered: Vec<domain::PaymentLink> = all_links
                    .into_iter()
                    .filter(|link| {
                        if let Some(ref filter) = status_filter {
                            &link.status == filter
                        } else {
                            true
                        }
                    })
                    .skip_while(|link| {
                        if let Some(ref c) = cursor {
                            link.payment_link_id.to_string() != *c
                        } else {
                            false
                        }
                    })
                    .take(page_limit)
                    .collect();

                let has_more = filtered.len() >= page_limit;
                let next_cursor = filtered.last().map(|link| link.payment_link_id.to_string());
                let views: Vec<PaymentLinkView> =
                    filtered.into_iter().map(payment_link_to_view).collect();

                Ok(Response::new(ListPaymentLinksResponse {
                    links: views,
                    pagination: Some(PaginationResponse {
                        next_cursor: next_cursor.unwrap_or_default(),
                        has_more,
                        as_of_unix_ms: chrono::Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(payment_link_error_to_status(e)),
        }
    }

    async fn expire_payment_link(
        &self,
        request: Request<ExpirePaymentLinkRequest>,
    ) -> Result<Response<ExpirePaymentLinkResponse>, Status> {
        let req = request.into_inner();
        let payment_link_id = parse_uuid(&req.payment_link_id, "payment_link_id")?;

        let cmd = commands::CancelPaymentLinkCommand {
            payment_link_id,
            reason: Some("expired via API".into()),
        };

        match self.commands.cancel_payment_link(cmd).await {
            Ok(link) => {
                Ok(Response::new(ExpirePaymentLinkResponse {
                    status: link.status.to_string(),
                }))
            }
            Err(e) => Err(payment_link_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn payment_link_to_view(link: domain::PaymentLink) -> PaymentLinkView {
    PaymentLinkView {
        payment_link_id: link.payment_link_id.to_string(),
        operator_id: link.operator_id.to_string(),
        amount: Some(ProtoMoney {
            amount_minor_units: link.amount_minor_units,
            currency_code: link.currency,
        }),
        description: link.description.unwrap_or_default(),
        status: link.status.to_string(),
        token: link.token,
        created_at: Some(Timestamp {
            unix_ms: link.created_at.timestamp_millis(),
        }),
        expires_at: Some(Timestamp {
            unix_ms: link.expires_at.timestamp_millis(),
        }),
        completed_at: link.used_at.map(|dt| Timestamp {
            unix_ms: dt.timestamp_millis(),
        }),
        payment_intent_id: link
            .payment_intent_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
    }
}

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn payment_link_error_to_status(e: PaymentLinkError) -> Status {
    match e {
        PaymentLinkError::NotFound => Status::not_found("Payment link not found"),
        PaymentLinkError::LinkExpired => Status::failed_precondition("Payment link has expired"),
        PaymentLinkError::LinkAlreadyUsed => {
            Status::failed_precondition("Payment link has already been used")
        }
        PaymentLinkError::LinkCancelled => {
            Status::failed_precondition("Payment link has been cancelled")
        }
        PaymentLinkError::InvalidLinkAmount => {
            Status::invalid_argument("Invalid link amount: must be positive")
        }
        PaymentLinkError::InvalidTransition => {
            Status::failed_precondition("Invalid status transition")
        }
    }
}

impl From<PaymentLinkError> for Status {
    fn from(e: PaymentLinkError) -> Self {
        payment_link_error_to_status(e)
    }
}
