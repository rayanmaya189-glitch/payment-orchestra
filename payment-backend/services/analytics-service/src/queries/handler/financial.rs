use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{AnalyticsQueryHandler, QueryHandler};
use crate::domain::*;
use crate::repository::*;

#[async_trait]
impl<R: AnalyticsRepository + Send + Sync> QueryHandler for AnalyticsQueryHandler<R> {
    async fn authorization_rates(&self, query: AuthRateQuery) -> Result<Vec<AuthRateRow>, AnalyticsError> {
        let events = self
            .repo
            .get_events_in_range(query.date_range.start, query.date_range.end)
            .await?;

        let mut rows: Vec<AuthRateRow> = Vec::new();
        let mut grouped: std::collections::HashMap<(i64, String, String), (u64, u64)> =
            std::collections::HashMap::new();

        for event in &events {
            let acquirer = event.acquirer_id.clone().unwrap_or_default();
            let scheme = event.card_scheme.clone().unwrap_or_default();

            if let Some(ref acquirer_ids) = query.acquirer_ids {
                if !acquirer_ids.contains(&acquirer) {
                    continue;
                }
            }
            if let Some(ref schemes) = query.card_schemes {
                if !schemes.contains(&scheme) {
                    continue;
                }
            }

            let hour_ts = event.occurred_at.timestamp() / 3600;
            let key = (hour_ts, acquirer, scheme);
            let entry = grouped.entry(key).or_insert((0, 0));

            match event.event_type.as_str() {
                "PaymentAuthorized" => entry.0 += 1,
                "PaymentFailed" => entry.1 += 1,
                _ => {}
            }
        }

        for ((hour_ts, acquirer, scheme), (approved, declined)) in grouped {
            let total = approved + declined;
            let rate = if total > 0 {
                (approved as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            let hour = DateTime::from_timestamp(hour_ts * 3600, 0).unwrap_or(Utc::now());
            rows.push(AuthRateRow {
                hour,
                acquirer_id: acquirer,
                card_scheme: scheme,
                approved_count: approved,
                declined_count: declined,
                total_count: total,
                auth_rate_pct: (rate * 100.0).round() / 100.0,
            });
        }

        rows.sort_by_key(|a| a.hour);
        Ok(rows)
    }

    async fn decline_reasons(&self, query: DeclineReasonQuery) -> Result<Vec<DeclineReasonRow>, AnalyticsError> {
        let events = self
            .repo
            .get_events_by_type("PaymentFailed", query.date_range.start, query.date_range.end)
            .await?;

        let mut reasons: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let total = events.len() as f64;

        for event in &events {
            if let Some(ref acquirer_ids) = query.acquirer_ids {
                if let Some(ref acquirer) = event.acquirer_id {
                    if !acquirer_ids.contains(acquirer) {
                        continue;
                    }
                }
            }

            let reason = event.decline_reason.clone().unwrap_or_else(|| "Unknown".into());
            *reasons.entry(reason).or_insert(0) += 1;
        }

        let mut rows: Vec<DeclineReasonRow> = reasons
            .into_iter()
            .map(|(reason, count)| DeclineReasonRow {
                decline_reason: reason,
                count,
                percentage_pct: if total > 0.0 {
                    ((count as f64 / total) * 10000.0).round() / 100.0
                } else {
                    0.0
                },
            })
            .collect();

        rows.sort_by_key(|a| std::cmp::Reverse(a.count));
        Ok(rows)
    }

    async fn settlement_status(&self, date_range: DateRangeFilter) -> Result<Vec<SettlementStatusRow>, AnalyticsError> {
        let matched = self
            .repo
            .get_events_by_type("SettlementMatched", date_range.start, date_range.end)
            .await?;
        let unmatched = self
            .repo
            .get_events_by_type("SettlementUnmatched", date_range.start, date_range.end)
            .await?;

        let matched_amount: i64 = matched.iter().filter_map(|e| e.amount_minor_units).sum();
        let unmatched_amount: i64 = unmatched.iter().filter_map(|e| e.amount_minor_units).sum();

        Ok(vec![
            SettlementStatusRow {
                status: "matched".into(),
                count: matched.len() as u64,
                total_amount_minor_units: matched_amount,
            },
            SettlementStatusRow {
                status: "unmatched".into(),
                count: unmatched.len() as u64,
                total_amount_minor_units: unmatched_amount,
            },
        ])
    }

    async fn fee_analysis(&self, query: FeeAnalysisQuery) -> Result<Vec<FeeAnalysisRow>, AnalyticsError> {
        let events = self
            .repo
            .get_events_by_type("FeeRecorded", query.date_range.start, query.date_range.end)
            .await?;

        let mut by_acquirer: std::collections::HashMap<String, (i64, u64)> =
            std::collections::HashMap::new();

        for event in &events {
            if let Some(ref acquirer_ids) = query.acquirer_ids {
                if let Some(ref acquirer) = event.acquirer_id {
                    if !acquirer_ids.contains(acquirer) {
                        continue;
                    }
                }
            }

            let acquirer = event.acquirer_id.clone().unwrap_or_else(|| "unknown".into());
            let fees = event.acquirer_fee.unwrap_or(0);
            let entry = by_acquirer.entry(acquirer).or_insert((0, 0));
            entry.0 += fees;
            entry.1 += 1;
        }

        let mut rows: Vec<FeeAnalysisRow> = by_acquirer
            .into_iter()
            .map(|(acquirer, (fees, count))| FeeAnalysisRow {
                acquirer_id: acquirer,
                total_fees_minor_units: fees,
                transaction_count: count,
                avg_fee_per_transaction: if count > 0 {
                    ((fees as f64 / count as f64) * 100.0).round() / 100.0
                } else {
                    0.0
                },
            })
            .collect();

        rows.sort_by_key(|a| std::cmp::Reverse(a.total_fees_minor_units));
        Ok(rows)
    }

    async fn chargeback_trends(&self, _query: ChargebackTrendQuery) -> Result<Vec<ChargebackTrendRow>, AnalyticsError> {
        unreachable!("implemented in fraud module")
    }

    async fn scheme_compliance(&self) -> Result<Vec<SchemeComplianceRow>, AnalyticsError> {
        unreachable!("implemented in fraud module")
    }

    async fn fraud_analysis(&self, _query: FraudAnalysisQuery) -> Result<Vec<FraudAnalysisRow>, AnalyticsError> {
        unreachable!("implemented in fraud module")
    }

    async fn revenue_recovery(&self, _date_range: DateRangeFilter) -> Result<Vec<RevenueRecoveryRow>, AnalyticsError> {
        unreachable!("implemented in other module")
    }

    async fn health_check(&self, _staleness_threshold_seconds: i64) -> Result<(), AnalyticsError> {
        unreachable!("implemented in other module")
    }
}
