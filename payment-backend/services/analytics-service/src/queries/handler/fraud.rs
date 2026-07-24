use async_trait::async_trait;
use chrono::{Duration, Utc};

use super::{AnalyticsQueryHandler, QueryHandler};
use crate::domain::*;
use crate::repository::*;

#[async_trait]
impl<R: AnalyticsRepository + Send + Sync> QueryHandler for AnalyticsQueryHandler<R> {
    async fn authorization_rates(&self, _query: AuthRateQuery) -> Result<Vec<AuthRateRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn decline_reasons(&self, _query: DeclineReasonQuery) -> Result<Vec<DeclineReasonRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn settlement_status(&self, _date_range: DateRangeFilter) -> Result<Vec<SettlementStatusRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn fee_analysis(&self, _query: FeeAnalysisQuery) -> Result<Vec<FeeAnalysisRow>, AnalyticsError> {
        unreachable!("implemented in financial module")
    }

    async fn chargeback_trends(&self, query: ChargebackTrendQuery) -> Result<Vec<ChargebackTrendRow>, AnalyticsError> {
        let now = Utc::now();
        let start = now - Duration::days(query.period_days as i64);

        let chargebacks = self
            .repo
            .get_events_by_type("ChargebackReceived", start, now)
            .await?;
        let all_events = self.repo.get_events_in_range(start, now).await?;

        let mut chargeback_count: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        let mut tx_count: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();

        for event in &chargebacks {
            if let Some(ref scheme) = event.card_scheme {
                *chargeback_count.entry(scheme.clone()).or_insert(0) += 1;
            }
        }

        for event in &all_events {
            if let Some(ref scheme) = event.card_scheme {
                match event.event_type.as_str() {
                    "PaymentAuthorized" | "PaymentFailed" => {
                        *tx_count.entry(scheme.clone()).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }

        let mut schemes: Vec<String> = chargeback_count
            .keys()
            .chain(tx_count.keys())
            .cloned()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if let Some(ref filter_schemes) = query.card_schemes {
            schemes.retain(|s| filter_schemes.contains(s));
        }

        let mut rows: Vec<ChargebackTrendRow> = schemes
            .into_iter()
            .map(|scheme| {
                let cb_count = chargeback_count.get(&scheme).copied().unwrap_or(0);
                let tx_total = tx_count.get(&scheme).copied().unwrap_or(0);
                let rate = if tx_total > 0 {
                    (cb_count as f64 / tx_total as f64) * 100.0
                } else {
                    0.0
                };
                ChargebackTrendRow {
                    card_scheme: scheme,
                    chargeback_count: cb_count,
                    transaction_count: tx_total,
                    chargeback_rate_pct: (rate * 100.0).round() / 100.0,
                    period_days: query.period_days,
                }
            })
            .collect();

        rows.sort_by(|a, b| b.chargeback_rate_pct.partial_cmp(&a.chargeback_rate_pct).unwrap());
        Ok(rows)
    }

    async fn scheme_compliance(&self) -> Result<Vec<SchemeComplianceRow>, AnalyticsError> {
        let now = Utc::now();
        let ninety_days_ago = now - Duration::days(90);

        let all_events = self.repo.get_events_in_range(ninety_days_ago, now).await?;
        let chargebacks = all_events
            .iter()
            .filter(|e| e.event_type == "ChargebackReceived");

        let mut by_scheme: std::collections::HashMap<String, (u64, u64, u64)> =
            std::collections::HashMap::new();

        for event in &all_events {
            if let Some(ref scheme) = event.card_scheme {
                let entry = by_scheme.entry(scheme.clone()).or_insert((0, 0, 0));
                match event.event_type.as_str() {
                    "PaymentAuthorized" => entry.0 += 1,
                    "PaymentFailed" => entry.1 += 1,
                    _ => {}
                }
            }
        }

        for event in chargebacks {
            if let Some(ref scheme) = event.card_scheme {
                let entry = by_scheme.entry(scheme.clone()).or_insert((0, 0, 0));
                entry.2 += 1;
            }
        }

        let thresholds: Vec<(&str, f64)> = vec![("visa", 1.5), ("mastercard", 1.8)];

        let mut rows: Vec<SchemeComplianceRow> = Vec::new();
        for &(scheme, threshold) in &thresholds {
            if let Some((auth, declined, cb_count)) = by_scheme.get(scheme) {
                let total_tx = *auth + *declined;
                let rate = if total_tx > 0 {
                    (*cb_count as f64 / total_tx as f64) * 100.0
                } else {
                    0.0
                };
                rows.push(SchemeComplianceRow {
                    card_scheme: scheme.into(),
                    threshold_name: "Chargeback Rate".into(),
                    current_rate_pct: (rate * 100.0).round() / 100.0,
                    threshold_pct: threshold,
                    is_breaching: rate > threshold,
                });
            } else {
                rows.push(SchemeComplianceRow {
                    card_scheme: scheme.into(),
                    threshold_name: "Chargeback Rate".into(),
                    current_rate_pct: 0.0,
                    threshold_pct: threshold,
                    is_breaching: false,
                });
            }
        }

        Ok(rows)
    }

    async fn fraud_analysis(&self, query: FraudAnalysisQuery) -> Result<Vec<FraudAnalysisRow>, AnalyticsError> {
        let all_events = self
            .repo
            .get_events_in_range(query.date_range.start, query.date_range.end)
            .await?;

        let mut dimension_map: std::collections::HashMap<String, (u64, u64)> =
            std::collections::HashMap::new();

        for event in &all_events {
            let dimension_val = match query.dimension {
                FraudDimension::Bin => event.bin.clone().unwrap_or_else(|| "Unknown".into()),
                FraudDimension::Country => event.country_code.clone().unwrap_or_else(|| "Unknown".into()),
                FraudDimension::AmountRange => {
                    match event.amount_minor_units {
                        Some(amt) if amt < 1000 => "0-10".into(),
                        Some(amt) if amt < 10000 => "10-100".into(),
                        Some(amt) if amt < 100000 => "100-1000".into(),
                        Some(amt) if amt < 1000000 => "1000-10000".into(),
                        Some(_) => "10000+".into(),
                        None => "Unknown".into(),
                    }
                }
            };

            let entry = dimension_map.entry(dimension_val).or_insert((0, 0));
            entry.0 += 1;

            let is_fraud = event
                .fraud_score
                .map(|s| s > 0.8)
                .unwrap_or(false)
                || event.event_type == "ChargebackReceived";
            if is_fraud {
                entry.1 += 1;
            }
        }

        let mut rows: Vec<FraudAnalysisRow> = dimension_map
            .into_iter()
            .map(|(val, (tx_count, fraud_count))| FraudAnalysisRow {
                dimension: match query.dimension {
                    FraudDimension::Bin => "BIN".into(),
                    FraudDimension::Country => "Country".into(),
                    FraudDimension::AmountRange => "Amount Range".into(),
                },
                dimension_value: val,
                transaction_count: tx_count,
                fraud_count,
                fraud_rate_pct: if tx_count > 0 {
                    ((fraud_count as f64 / tx_count as f64) * 10000.0).round() / 100.0
                } else {
                    0.0
                },
            })
            .collect();

        rows.sort_by(|a, b| b.fraud_rate_pct.partial_cmp(&a.fraud_rate_pct).unwrap());
        Ok(rows)
    }

    async fn revenue_recovery(&self, _date_range: DateRangeFilter) -> Result<Vec<RevenueRecoveryRow>, AnalyticsError> {
        unreachable!("implemented in other module")
    }

    async fn health_check(&self, _staleness_threshold_seconds: i64) -> Result<(), AnalyticsError> {
        unreachable!("implemented in other module")
    }
}
