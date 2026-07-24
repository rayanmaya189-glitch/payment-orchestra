//! Analytics Service query handlers
//!
//! 8 read endpoints matching the spec:
//! 1. Authorization rates (hourly per acquirer/scheme)
//! 2. Decline reason breakdown
//! 3. Settlement status (matched/unmatched counts)
//! 4. Fee analysis (per acquirer)
//! 5. Chargeback trends (30/90 day by scheme)
//! 6. Scheme compliance (Visa/Mastercard thresholds)
//! 7. Fraud analysis (by BIN, geography, amount)
//! 8. Revenue recovery (via failover)

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use crate::domain::*;
use crate::repository::*;

// ---------------------------------------------------------------------------
// QueryHandler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait QueryHandler: Send + Sync {
    /// Hourly authorization rates per acquirer/card scheme.
    async fn authorization_rates(&self, query: AuthRateQuery) -> Result<Vec<AuthRateRow>, AnalyticsError>;
    /// Decline reason breakdown.
    async fn decline_reasons(&self, query: DeclineReasonQuery) -> Result<Vec<DeclineReasonRow>, AnalyticsError>;
    /// Settlement matched/unmatched counts.
    async fn settlement_status(&self, date_range: DateRangeFilter) -> Result<Vec<SettlementStatusRow>, AnalyticsError>;
    /// Fees per acquirer.
    async fn fee_analysis(&self, query: FeeAnalysisQuery) -> Result<Vec<FeeAnalysisRow>, AnalyticsError>;
    /// Chargeback rate by scheme (30/90 day).
    async fn chargeback_trends(&self, query: ChargebackTrendQuery) -> Result<Vec<ChargebackTrendRow>, AnalyticsError>;
    /// Visa/Mastercard threshold monitoring.
    async fn scheme_compliance(&self) -> Result<Vec<SchemeComplianceRow>, AnalyticsError>;
    /// Fraud rate by BIN, geography, amount.
    async fn fraud_analysis(&self, query: FraudAnalysisQuery) -> Result<Vec<FraudAnalysisRow>, AnalyticsError>;
    /// Revenue recovered via failover.
    async fn revenue_recovery(&self, date_range: DateRangeFilter) -> Result<Vec<RevenueRecoveryRow>, AnalyticsError>;
    /// Health check — verify data is fresh.
    async fn health_check(&self, staleness_threshold_seconds: i64) -> Result<(), AnalyticsError>;
}

// ---------------------------------------------------------------------------
// AnalyticsQueryHandler
// ---------------------------------------------------------------------------

pub struct AnalyticsQueryHandler<R: AnalyticsRepository> {
    repo: R,
}

impl<R: AnalyticsRepository> AnalyticsQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

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

    async fn revenue_recovery(&self, date_range: DateRangeFilter) -> Result<Vec<RevenueRecoveryRow>, AnalyticsError> {
        let routing_events = self
            .repo
            .get_events_by_type("RoutingDecision", date_range.start, date_range.end)
            .await?;

        let mut failover_count: u64 = 0;
        let mut recovered_amount: i64 = 0;

        for event in &routing_events {
            if event.failover_routed.unwrap_or(false) {
                failover_count += 1;
                recovered_amount += event.amount_minor_units.unwrap_or(0);
            }
        }

        let estimated_savings = (recovered_amount as f64 * 0.02) as i64;

        Ok(vec![
            RevenueRecoveryRow {
                period: format!(
                    "{} - {}",
                    date_range.start.format("%Y-%m-%d"),
                    date_range.end.format("%Y-%m-%d")
                ),
                failover_count,
                recovered_amount_minor_units: recovered_amount,
                estimated_savings_minor_units: estimated_savings,
                currency: "USD".into(),
            },
        ])
    }

    async fn health_check(&self, staleness_threshold_seconds: i64) -> Result<(), AnalyticsError> {
        match self.repo.last_ingested_at().await {
            Some(last) => {
                let elapsed = (Utc::now() - last).num_seconds();
                if elapsed > staleness_threshold_seconds {
                    return Err(AnalyticsError::StaleData(elapsed));
                }
                Ok(())
            }
            None => {
                Ok(())
            }
        }
    }
}
