#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

/// Analytics metric types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    AuthorizationRate,
    DeclineRate,
    TransactionVolume,
    SettlementStatus,
    FeeAnalysis,
    ChargebackRate,
    AverageLatency,
    SuccessRate,
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AuthorizationRate => "authorization_rate",
            Self::DeclineRate => "decline_rate",
            Self::TransactionVolume => "transaction_volume",
            Self::SettlementStatus => "settlement_status",
            Self::FeeAnalysis => "fee_analysis",
            Self::ChargebackRate => "chargeback_rate",
            Self::AverageLatency => "average_latency",
            Self::SuccessRate => "success_rate",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "authorization_rate" => Ok(Self::AuthorizationRate),
            "decline_rate" => Ok(Self::DeclineRate),
            "transaction_volume" => Ok(Self::TransactionVolume),
            "settlement_status" => Ok(Self::SettlementStatus),
            "fee_analysis" => Ok(Self::FeeAnalysis),
            "chargeback_rate" => Ok(Self::ChargebackRate),
            "average_latency" => Ok(Self::AverageLatency),
            "success_rate" => Ok(Self::SuccessRate),
            _ => Err("unknown metric type"),
        }
    }
}

/// Time granularity for analytics queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeGranularity {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

impl TimeGranularity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }
}

/// Export format for analytics reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Json,
    Pdf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type_values() {
        assert_eq!(MetricType::AuthorizationRate.as_str(), "authorization_rate");
        assert_eq!(MetricType::TransactionVolume.as_str(), "transaction_volume");
    }

    #[test]
    fn test_metric_type_from_str() {
        assert_eq!(MetricType::from_str("authorization_rate").unwrap(), MetricType::AuthorizationRate);
        assert!(MetricType::from_str("unknown").is_err());
    }

    #[test]
    fn test_granularity_values() {
        assert_eq!(TimeGranularity::Hourly.as_str(), "hourly");
        assert_eq!(TimeGranularity::Monthly.as_str(), "monthly");
    }
}
