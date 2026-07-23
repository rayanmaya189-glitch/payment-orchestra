//! gRPC service implementation for subscription-service (BC-08).
//! Translates between protobuf types and domain types for subscription lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{
    CommandHandler, CreateSubscriptionCommand, CancelSubscriptionCommand,
    PauseSubscriptionCommand, ResumeSubscriptionCommand,
};
use crate::domain::{self, SubscriptionError, SubscriptionPlan, SubscriptionStatus};

use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationResponse};
use platform_proto::subscription::subscription_service_server::SubscriptionService;
use platform_proto::subscription::*;

/// Convert billing interval string to days.
fn billing_interval_to_days(interval: &str) -> Result<i64, Status> {
    match interval {
        "weekly" => Ok(7),
        "monthly" => Ok(30),
        "yearly" => Ok(365),
        other => Err(Status::invalid_argument(format!(
            "Invalid billing_interval '{}': expected 'weekly', 'monthly', or 'yearly'",
            other
        ))),
    }
}

/// Convert days back to billing interval string.
fn days_to_billing_interval(days: i64) -> String {
    match days {
        7 => "weekly".to_string(),
        30 => "monthly".to_string(),
        365 => "yearly".to_string(),
        other => format!("{}d", other),
    }
}

pub struct SubscriptionGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> SubscriptionGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> SubscriptionService for SubscriptionGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: crate::queries::QueryHandler + Send + Sync + 'static,
{
    async fn create_subscription(
        &self,
        request: Request<CreateSubscriptionRequest>,
    ) -> Result<Response<CreateSubscriptionResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;
        let customer_id = parse_uuid(&req.customer_id, "customer_id")?;
        let payment_method_token_id = if req.payment_method_token_id.is_empty() {
            None
        } else {
            Some(parse_uuid(&req.payment_method_token_id, "payment_method_token_id")?)
        };

        let billing_interval_days = billing_interval_to_days(&req.billing_interval)?;
        let amount = req.amount.ok_or_else(|| Status::invalid_argument("amount is required"))?;

        let plan = SubscriptionPlan {
            plan_id: req.plan_id,
            name: String::new(), // not available from proto, leave empty
            amount_minor_units: amount.amount_minor_units,
            currency: amount.currency_code,
            billing_interval_days,
            trial_period_days: Some(req.trial_days as i64).filter(|&d| d > 0),
            is_active: true,
        };

        let cmd = CreateSubscriptionCommand {
            operator_id,
            customer_id,
            plan,
            payment_method_token_id,
            max_dunning_retries: Some(3), // default
        };

        match self.commands.create_subscription(cmd).await {
            Ok(subscription) => {
                Ok(Response::new(CreateSubscriptionResponse {
                    subscription_id: subscription.subscription_id.to_string(),
                    status: subscription.status.to_string(),
                    current_period_start: Some(Timestamp {
                        unix_ms: subscription.current_period_start.timestamp_millis(),
                    }),
                    current_period_end: Some(Timestamp {
                        unix_ms: subscription.current_period_end.timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(subscription_error_to_status(e)),
        }
    }

    async fn get_subscription(
        &self,
        request: Request<GetSubscriptionRequest>,
    ) -> Result<Response<SubscriptionView>, Status> {
        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id, "subscription_id")?;

        match self.queries.get_subscription(subscription_id).await {
            Ok(sub) => Ok(Response::new(subscription_to_view(sub))),
            Err(e) => Err(subscription_error_to_status(e)),
        }
    }

    async fn cancel_subscription(
        &self,
        request: Request<CancelSubscriptionRequest>,
    ) -> Result<Response<CancelSubscriptionResponse>, Status> {
        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id, "subscription_id")?;
        let reason = if req.reason.is_empty() { None } else { Some(req.reason) };

        // If cancel_at_period_end is true, the subscription remains active until
        // the current period ends. Our domain model cancels immediately, so for
        // cancel_at_period_end we report the current period end as effective_end.
        let cmd = CancelSubscriptionCommand {
            subscription_id,
            reason,
            has_running_renewal: false,
        };

        match self.commands.cancel_subscription(cmd).await {
            Ok(sub) => {
                let effective_end = if req.cancel_at_period_end {
                    sub.current_period_end
                } else {
                    sub.cancelled_at.unwrap_or_else(chrono::Utc::now)
                };

                Ok(Response::new(CancelSubscriptionResponse {
                    status: sub.status.to_string(),
                    effective_end: Some(Timestamp {
                        unix_ms: effective_end.timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(subscription_error_to_status(e)),
        }
    }

    async fn pause_subscription(
        &self,
        request: Request<PauseSubscriptionRequest>,
    ) -> Result<Response<PauseSubscriptionResponse>, Status> {
        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id, "subscription_id")?;

        let cmd = PauseSubscriptionCommand { subscription_id };

        match self.commands.pause_subscription(cmd).await {
            Ok(sub) => {
                Ok(Response::new(PauseSubscriptionResponse {
                    status: sub.status.to_string(),
                }))
            }
            Err(e) => Err(subscription_error_to_status(e)),
        }
    }

    async fn resume_subscription(
        &self,
        request: Request<ResumeSubscriptionRequest>,
    ) -> Result<Response<ResumeSubscriptionResponse>, Status> {
        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id, "subscription_id")?;

        let cmd = ResumeSubscriptionCommand { subscription_id };

        match self.commands.resume_subscription(cmd).await {
            Ok(sub) => {
                Ok(Response::new(ResumeSubscriptionResponse {
                    status: sub.status.to_string(),
                }))
            }
            Err(e) => Err(subscription_error_to_status(e)),
        }
    }

    async fn list_subscriptions(
        &self,
        request: Request<ListSubscriptionsRequest>,
    ) -> Result<Response<ListSubscriptionsResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        let status_filter = if req.status_filter.is_empty() {
            None
        } else {
            Some(req.status_filter.parse::<SubscriptionStatus>().map_err(|_| {
                Status::invalid_argument(format!("Invalid status filter: {}", req.status_filter))
            })?)
        };

        let page_limit = req.pagination.as_ref().map_or(50, |p| {
            if p.limit > 0 && p.limit <= 100 { p.limit as usize } else { 50 }
        });
        let cursor = req.pagination.as_ref().and_then(|p| {
            if p.cursor.is_empty() { None } else { Some(p.cursor.clone()) }
        });

        // Use find_by_operator to get all subscriptions, then filter client-side
        match self.queries.find_by_operator(operator_id).await {
            Ok(all_subs) => {
                let filtered: Vec<domain::Subscription> = all_subs.into_iter()
                    .filter(|sub| {
                        if let Some(ref filter) = status_filter {
                            &sub.status == filter
                        } else {
                            true
                        }
                    })
                    .skip_while(|sub| {
                        if let Some(ref c) = cursor {
                            sub.subscription_id.to_string() != *c
                        } else {
                            false
                        }
                    })
                    .take(page_limit)
                    .collect();

                let has_more = filtered.len() >= page_limit;
                let next_cursor = filtered.last().map(|sub| sub.subscription_id.to_string());
                let views: Vec<SubscriptionView> = filtered.into_iter()
                    .map(subscription_to_view)
                    .collect();

                Ok(Response::new(ListSubscriptionsResponse {
                    subscriptions: views,
                    pagination: Some(PaginationResponse {
                        next_cursor: next_cursor.unwrap_or_default(),
                        has_more,
                        as_of_unix_ms: chrono::Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(subscription_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn subscription_to_view(sub: domain::Subscription) -> SubscriptionView {
    SubscriptionView {
        subscription_id: sub.subscription_id.to_string(),
        operator_id: sub.operator_id.to_string(),
        customer_id: sub.customer_id.to_string(),
        plan_id: sub.plan_id,
        status: sub.status.to_string(),
        amount: Some(ProtoMoney {
            amount_minor_units: sub.plan_amount_minor_units,
            currency_code: sub.currency,
        }),
        billing_interval: days_to_billing_interval(sub.billing_interval_days),
        payment_method_token_id: sub.payment_method_token_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        current_period_start: Some(Timestamp {
            unix_ms: sub.current_period_start.timestamp_millis(),
        }),
        current_period_end: Some(Timestamp {
            unix_ms: sub.current_period_end.timestamp_millis(),
        }),
        dunning_retry_count: sub.dunning_retry_count,
        created_at: Some(Timestamp {
            unix_ms: sub.created_at.timestamp_millis(),
        }),
        cancelled_at: sub.cancelled_at.map(|dt| Timestamp {
            unix_ms: dt.timestamp_millis(),
        }),
    }
}

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn subscription_error_to_status(e: SubscriptionError) -> Status {
    match e {
        SubscriptionError::NotFound(id) => {
            Status::not_found(format!("Subscription not found: {}", id))
        }
        SubscriptionError::AlreadyCancelled => {
            Status::failed_precondition("Subscription already cancelled")
        }
        SubscriptionError::CannotCancelDuringRenewal => {
            Status::failed_precondition("Cannot cancel subscription during renewal")
        }
        SubscriptionError::InvalidPlanAmount => {
            Status::invalid_argument("Invalid plan amount: must be positive")
        }
        SubscriptionError::InvalidTransition => {
            Status::failed_precondition("Invalid subscription status transition")
        }
        SubscriptionError::DunningExhausted => {
            Status::failed_precondition("Dunning retries exhausted")
        }
        SubscriptionError::BillingCycleNotFound => {
            Status::not_found("Billing cycle not found")
        }
    }
}

impl From<SubscriptionError> for Status {
    fn from(e: SubscriptionError) -> Self {
        subscription_error_to_status(e)
    }
}
