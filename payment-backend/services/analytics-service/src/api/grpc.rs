//! gRPC service implementation for analytics-service (BC-15).
//! Translates between protobuf types and domain types for analytics queries.

use chrono::{DateTime, Datelike, Utc};
use tonic::{Request, Response, Status};

use crate::commands::CommandHandler;
use crate::domain::*;
use crate::queries::QueryHandler;
use crate::repository::AnalyticsRepository;

use platform_proto::analytics::analytics_service_server::AnalyticsService;
use platform_proto::analytics::*;
use platform_proto::common::Money as ProtoMoney;

pub struct AnalyticsGrpcService<C, Q, R> {
    _commands: C,
    queries: Q,
    repo: R,
}

impl<C, Q, R> AnalyticsGrpcService<C, Q, R> {
    pub fn new(commands: C, queries: Q, repo: R) -> Self {
        Self { _commands: commands, queries, repo }
    }
}

#[tonic::async_trait]
impl<C, Q, R> AnalyticsService for AnalyticsGrpcService<C, Q, R>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
    R: AnalyticsRepository + Send + Sync + 'static,
{
    async fn get_authorization_rates(
        &self,
        request: Request<GetAuthRatesRequest>,
    ) -> Result<Response<GetAuthRatesResponse>, Status> {
        let req = request.into_inner();
        let start = parse_timestamp(req.start_unix_ms, "start_unix_ms")?;
        let end = parse_timestamp(req.end_unix_ms, "end_unix_ms")?;

        let query = AuthRateQuery {
            date_range: DateRangeFilter { start, end },
            acquirer_ids: if req.acquirer_link_id.is_empty() {
                None
            } else {
                Some(vec![req.acquirer_link_id])
            },
            card_schemes: None,
        };

        match self.queries.authorization_rates(query).await {
            Ok(rows) => {
                let total_approved: u64 = rows.iter().map(|r| r.approved_count).sum();
                let total_declined: u64 = rows.iter().map(|r| r.declined_count).sum();
                let total = total_approved + total_declined;
                let overall_rate = if total > 0 {
                    (total_approved as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                let rates: Vec<AuthRateByAcquirer> = rows
                    .into_iter()
                    .map(|r| AuthRateByAcquirer {
                        acquirer_name: r.acquirer_id,
                        total_attempts: r.total_count as i64,
                        approved: r.approved_count as i64,
                        declined: r.declined_count as i64,
                        auth_rate: r.auth_rate_pct,
                        avg_latency_ms: 0.0, // Not available from domain model
                    })
                    .collect();

                Ok(Response::new(GetAuthRatesResponse {
                    rates,
                    overall_auth_rate: (overall_rate * 100.0).round() / 100.0,
                }))
            }
            Err(e) => Err(analytics_error_to_status(e)),
        }
    }

    async fn get_decline_reasons(
        &self,
        request: Request<GetDeclineReasonsRequest>,
    ) -> Result<Response<GetDeclineReasonsResponse>, Status> {
        let req = request.into_inner();
        let start = parse_timestamp(req.start_unix_ms, "start_unix_ms")?;
        let end = parse_timestamp(req.end_unix_ms, "end_unix_ms")?;

        let query = DeclineReasonQuery {
            date_range: DateRangeFilter { start, end },
            acquirer_ids: if req.acquirer_link_id.is_empty() {
                None
            } else {
                Some(vec![req.acquirer_link_id])
            },
        };

        match self.queries.decline_reasons(query).await {
            Ok(rows) => {
                let reasons: Vec<DeclineReasonDistribution> = rows
                    .into_iter()
                    .map(|r| DeclineReasonDistribution {
                        reason_code: r.decline_reason,
                        count: r.count as i64,
                        percentage: r.percentage_pct,
                    })
                    .collect();

                Ok(Response::new(GetDeclineReasonsResponse { reasons }))
            }
            Err(e) => Err(analytics_error_to_status(e)),
        }
    }

    async fn get_settlement_status(
        &self,
        request: Request<GetSettlementStatusRequest>,
    ) -> Result<Response<GetSettlementStatusResponse>, Status> {
        let req = request.into_inner();
        let start = parse_timestamp(req.start_unix_ms, "start_unix_ms")?;
        let end = parse_timestamp(req.end_unix_ms, "end_unix_ms")?;

        match self
            .queries
            .settlement_status(DateRangeFilter { start, end })
            .await
        {
            Ok(rows) => {
                let mut total_settled: i64 = 0;
                let mut total_pending: i64 = 0;
                let mut total_exceptions: i64 = 0;

                for row in &rows {
                    match row.status.as_str() {
                        "matched" => total_settled = row.count as i64,
                        "unmatched" => total_exceptions = row.count as i64,
                        "pending" => total_pending = row.count as i64,
                        _ => {}
                    }
                }

                let total_tx = total_settled + total_pending + total_exceptions;
                let settlement_rate = if total_tx > 0 {
                    (total_settled as f64 / total_tx as f64) * 100.0
                } else {
                    0.0
                };

                Ok(Response::new(GetSettlementStatusResponse {
                    total_settled,
                    total_pending,
                    total_exceptions,
                    settlement_rate: (settlement_rate * 100.0).round() / 100.0,
                }))
            }
            Err(e) => Err(analytics_error_to_status(e)),
        }
    }

    async fn get_fee_analysis(
        &self,
        request: Request<GetFeeAnalysisRequest>,
    ) -> Result<Response<GetFeeAnalysisResponse>, Status> {
        let req = request.into_inner();
        let start = parse_timestamp(req.start_unix_ms, "start_unix_ms")?;
        let end = parse_timestamp(req.end_unix_ms, "end_unix_ms")?;

        let query = FeeAnalysisQuery {
            date_range: DateRangeFilter { start, end },
            acquirer_ids: if req.acquirer_link_id.is_empty() {
                None
            } else {
                Some(vec![req.acquirer_link_id])
            },
        };

        // Fetch events to determine the most common currency in the data
        let events = self.repo.get_events_in_range(start, end).await;
        let currency = events.as_ref().map_or("AED".to_string(), |e| resolve_currency(e));

        match self.queries.fee_analysis(query).await {
            Ok(rows) => {
                let mut total_fees_minor: i64 = 0;
                let mut total_tx_count: u64 = 0;

                let fees: Vec<FeeByAcquirer> = rows
                    .into_iter()
                    .map(|r| {
                        total_fees_minor += r.total_fees_minor_units;
                        total_tx_count += r.transaction_count;
                        FeeByAcquirer {
                            acquirer_name: r.acquirer_id,
                            total_fees: Some(ProtoMoney {
                                amount_minor_units: r.total_fees_minor_units,
                                currency_code: currency.clone(),
                            }),
                            transaction_count: r.transaction_count as i64,
                            effective_rate_bps: if r.transaction_count > 0 {
                                ((r.total_fees_minor_units as f64 / r.transaction_count as f64)
                                    / 100.0)
                                    * 10000.0
                            } else {
                                0.0
                            },
                        }
                    })
                    .collect();

                let effective_rate = if total_tx_count > 0 {
                    ((total_fees_minor as f64 / total_tx_count as f64) / 100.0) * 10000.0
                } else {
                    0.0
                };

                Ok(Response::new(GetFeeAnalysisResponse {
                    fees,
                    total_fees: Some(ProtoMoney {
                        amount_minor_units: total_fees_minor,
                        currency_code: currency,
                    }),
                    effective_rate_bps: (effective_rate * 100.0).round() / 100.0,
                }))
            }
            Err(e) => Err(analytics_error_to_status(e)),
        }
    }

    async fn get_chargeback_trends(
        &self,
        request: Request<GetChargebackTrendsRequest>,
    ) -> Result<Response<GetChargebackTrendsResponse>, Status> {
        let req = request.into_inner();
        let start = parse_timestamp(req.start_unix_ms, "start_unix_ms")?;
        let end = parse_timestamp(req.end_unix_ms, "end_unix_ms")?;

        // Compute period in days from the date range
        let period_days = (end - start).num_days().max(1) as u32;

        let query = ChargebackTrendQuery {
            period_days,
            card_schemes: None,
        };

        match self.queries.chargeback_trends(query).await {
            Ok(rows) => {
                let total_chargebacks: i64 = rows.iter().map(|r| r.chargeback_count as i64).sum();
                let total_won: i64 = 0; // Not available from domain model
                let total_lost: i64 = 0; // Not available from domain model
                let total_tx: i64 = rows.iter().map(|r| r.transaction_count as i64).sum();
                let chargeback_rate = if total_tx > 0 {
                    (total_chargebacks as f64 / total_tx as f64) * 100.0
                } else {
                    0.0
                };

                Ok(Response::new(GetChargebackTrendsResponse {
                    total_chargebacks,
                    won: total_won,
                    lost: total_lost,
                    chargeback_rate: (chargeback_rate * 100.0).round() / 100.0,
                }))
            }
            Err(e) => Err(analytics_error_to_status(e)),
        }
    }

    async fn get_volume_over_time(
        &self,
        request: Request<GetVolumeOverTimeRequest>,
    ) -> Result<Response<GetVolumeOverTimeResponse>, Status> {
        let req = request.into_inner();
        let start = parse_timestamp(req.start_unix_ms, "start_unix_ms")?;
        let end = parse_timestamp(req.end_unix_ms, "end_unix_ms")?;

        // Fetch events from repo and group by the requested granularity
        match self.repo.get_events_in_range(start, end).await {
            Ok(events) => {
                let granularity = if req.granularity.is_empty() {
                    "day"
                } else {
                    &req.granularity
                };

                // Group by time bucket
                let mut buckets: std::collections::HashMap<i64, (i64, i64)> =
                    std::collections::HashMap::new();

                for event in &events {
                    let bucket = match granularity {
                        "hour" => event.occurred_at.timestamp() / 3600 * 3600,
                        "day" => {
                            let dt = event.occurred_at.date_naive();
                            dt.and_hms_opt(0, 0, 0)
                                .map(|d| d.and_utc().timestamp_millis())
                                .unwrap_or(0)
                        }
                        "week" => {
                            let dt = event.occurred_at.date_naive();
                            let iso = dt.iso_week();
                            let week_start = chrono::NaiveDate::from_isoywd_opt(
                                iso.year(),
                                iso.week(),
                                chrono::Weekday::Mon,
                            )
                            .unwrap_or(dt);
                            week_start
                                .and_hms_opt(0, 0, 0)
                                .map(|d| d.and_utc().timestamp_millis())
                                .unwrap_or(0)
                        }
                        "month" => {
                            let dt = event.occurred_at.date_naive();
                            let month_start =
                                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                                    .unwrap_or(dt);
                            month_start
                                .and_hms_opt(0, 0, 0)
                                .map(|d| d.and_utc().timestamp_millis())
                                .unwrap_or(0)
                        }
                        _ => event.occurred_at.timestamp_millis() / 86400000 * 86400000,
                    };

                    let entry = buckets.entry(bucket).or_insert((0, 0));
                    entry.0 += event.amount_minor_units.unwrap_or(0);
                    entry.1 += 1;
                }

                let currency = resolve_currency(&events);

                let mut points: Vec<VolumePoint> = buckets
                    .into_iter()
                    .map(|(ts, (volume, count))| VolumePoint {
                        timestamp_unix_ms: ts,
                        volume: Some(ProtoMoney {
                            amount_minor_units: volume,
                            currency_code: currency.clone(),
                        }),
                        transaction_count: count,
                    })
                    .collect();

                points.sort_by(|a, b| a.timestamp_unix_ms.cmp(&b.timestamp_unix_ms));

                Ok(Response::new(GetVolumeOverTimeResponse { points }))
            }
            Err(e) => Err(analytics_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Determine the most common currency from analytics events.
/// Falls back to "AED" if no events have a currency set.
fn resolve_currency(events: &[AnalyticsEvent]) -> String {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for event in events {
        if let Some(ref currency) = event.currency {
            *counts.entry(currency.as_str()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(currency, _)| currency.to_string())
        .unwrap_or_else(|| "AED".to_string())
}

fn parse_timestamp(unix_ms: i64, field: &str) -> Result<DateTime<Utc>, Status> {
    chrono::DateTime::from_timestamp_millis(unix_ms)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid {} timestamp", field)))
}

fn analytics_error_to_status(e: AnalyticsError) -> Status {
    match e {
        AnalyticsError::Unavailable(msg) => Status::unavailable(msg),
        AnalyticsError::StaleData(secs) => {
            Status::unavailable(format!("Stale data: last ingestion was {} seconds ago", secs))
        }
        AnalyticsError::InvalidDateRange(msg) => Status::invalid_argument(msg),
        AnalyticsError::QueryTimeout(msg) => Status::deadline_exceeded(msg),
        AnalyticsError::EventNotFound(id) => {
            Status::not_found(format!("Event not found: {}", id))
        }
        AnalyticsError::InvalidEventType(et) => {
            Status::invalid_argument(format!("Invalid event type: {}", et))
        }
    }
}

impl From<AnalyticsError> for Status {
    fn from(e: AnalyticsError) -> Self {
        analytics_error_to_status(e)
    }
}
