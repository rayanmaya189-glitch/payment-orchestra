use tonic::{Request, Response, Status};

use super::{AnalyticsGrpcService, parse_timestamp, analytics_error_to_status, resolve_currency};
use crate::commands::CommandHandler;
use crate::domain::*;
use crate::queries::QueryHandler;
use crate::repository::AnalyticsRepository;

use platform_proto::analytics::analytics_service_server::AnalyticsService;
use platform_proto::analytics::*;
use platform_proto::common::Money as ProtoMoney;

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
                        avg_latency_ms: 0.0,
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
        _request: Request<GetChargebackTrendsRequest>,
    ) -> Result<Response<GetChargebackTrendsResponse>, Status> {
        unreachable!("implemented in fraud module")
    }

    async fn get_volume_over_time(
        &self,
        _request: Request<GetVolumeOverTimeRequest>,
    ) -> Result<Response<GetVolumeOverTimeResponse>, Status> {
        unreachable!("implemented in fraud module")
    }
}
