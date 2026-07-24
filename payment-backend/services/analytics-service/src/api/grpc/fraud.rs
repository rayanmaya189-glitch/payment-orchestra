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
        _request: Request<GetAuthRatesRequest>,
    ) -> Result<Response<GetAuthRatesResponse>, Status> {
        unreachable!("implemented in financial module")
    }

    async fn get_decline_reasons(
        &self,
        _request: Request<GetDeclineReasonsRequest>,
    ) -> Result<Response<GetDeclineReasonsResponse>, Status> {
        unreachable!("implemented in financial module")
    }

    async fn get_settlement_status(
        &self,
        _request: Request<GetSettlementStatusRequest>,
    ) -> Result<Response<GetSettlementStatusResponse>, Status> {
        unreachable!("implemented in financial module")
    }

    async fn get_fee_analysis(
        &self,
        _request: Request<GetFeeAnalysisRequest>,
    ) -> Result<Response<GetFeeAnalysisResponse>, Status> {
        unreachable!("implemented in financial module")
    }

    async fn get_chargeback_trends(
        &self,
        request: Request<GetChargebackTrendsRequest>,
    ) -> Result<Response<GetChargebackTrendsResponse>, Status> {
        let req = request.into_inner();
        let start = parse_timestamp(req.start_unix_ms, "start_unix_ms")?;
        let end = parse_timestamp(req.end_unix_ms, "end_unix_ms")?;

        let period_days = (end - start).num_days().max(1) as u32;

        let query = ChargebackTrendQuery {
            period_days,
            card_schemes: None,
        };

        match self.queries.chargeback_trends(query).await {
            Ok(rows) => {
                let total_chargebacks: i64 = rows.iter().map(|r| r.chargeback_count as i64).sum();
                let total_won: i64 = 0;
                let total_lost: i64 = 0;
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

        match self.repo.get_events_in_range(start, end).await {
            Ok(events) => {
                let granularity = if req.granularity.is_empty() {
                    "day"
                } else {
                    &req.granularity
                };

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

                points.sort_by_key(|a| a.timestamp_unix_ms);

                Ok(Response::new(GetVolumeOverTimeResponse { points }))
            }
            Err(e) => Err(analytics_error_to_status(e)),
        }
    }
}
