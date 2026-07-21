//! Settlement file parser — parses settlement files from payment connectors.
//!
//! Supports multiple formats per SRS Part 7: CSV, JSON, and fixed-width.
//! Each connector may provide settlement files in different formats.

use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

use crate::domain::value_objects::SettlementRecord;

/// Supported settlement file formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementFileFormat {
    Csv,
    Json,
    FixedWidth,
}

/// Parse a settlement file and extract records.
pub fn parse_settlement_file(
    content: &str,
    format: SettlementFileFormat,
    connector_id: &str,
) -> Result<Vec<SettlementRecord>, PlatformError> {
    match format {
        SettlementFileFormat::Csv => parse_csv(content, connector_id),
        SettlementFileFormat::Json => parse_json(content, connector_id),
        SettlementFileFormat::FixedWidth => parse_fixed_width(content, connector_id),
    }
}

/// Parse CSV settlement file.
///
/// Expected columns: acquirer_reference, amount, currency, settled_at, fee, status
fn parse_csv(content: &str, connector_id: &str) -> Result<Vec<SettlementRecord>, PlatformError> {
    let mut records = Vec::new();
    let mut lines = content.lines();

    let _header = lines.next().ok_or_else(|| {
        PlatformError::Validation(platform_error::ValidationError::MissingField(
            "Empty CSV file".into(),
        ))
    })?;

    for (line_num, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if fields.len() < 5 {
            tracing::warn!(line = line_num + 2, "Skipping malformed CSV row");
            continue;
        }

        let acquirer_reference = fields[0].to_string();
        let amount = fields[1].parse::<i64>().unwrap_or(0);
        let currency = CurrencyCode::new(fields[2])
            .unwrap_or_else(|_| CurrencyCode::new("AED").unwrap());
        let settled_at = chrono::DateTime::parse_from_rfc3339(fields[3])
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let fee = fields[4].parse::<i64>().ok();

        records.push(SettlementRecord {
            acquirer_reference,
            amount: Money { amount_minor_units: amount, currency },
            settled_at,
            fee: fee.map(|f| Money {
                amount_minor_units: f,
                currency: CurrencyCode::new("AED").unwrap(),
            }),
            connector_id: connector_id.to_string(),
            status: fields.get(5).map(|s| s.to_string()).unwrap_or_else(|| "settled".to_string()),
        });
    }

    Ok(records)
}

/// Parse JSON settlement file.
fn parse_json(content: &str, connector_id: &str) -> Result<Vec<SettlementRecord>, PlatformError> {
    let parsed: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        PlatformError::Validation(platform_error::ValidationError::MissingField(format!(
            "Invalid JSON: {e}"
        )))
    })?;

    let arr = parsed.as_array().ok_or_else(|| {
        PlatformError::Validation(platform_error::ValidationError::MissingField(
            "JSON must be an array".into(),
        ))
    })?;

    let mut records = Vec::new();
    for (idx, item) in arr.iter().enumerate() {
        let acquirer_reference = item["transaction_id"].as_str().unwrap_or("").to_string();
        if acquirer_reference.is_empty() {
            tracing::warn!(index = idx, "Skipping JSON record without transaction_id");
            continue;
        }

        let amount = item["amount"].as_i64().unwrap_or(0);
        let currency_str = item["currency"].as_str().unwrap_or("AED");
        let currency =
            CurrencyCode::new(currency_str).unwrap_or_else(|_| CurrencyCode::new("AED").unwrap());
        let settled_at = item["settled_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|| chrono::Utc::now());
        let fee = item["fee"].as_i64();

        records.push(SettlementRecord {
            acquirer_reference,
            amount: Money { amount_minor_units: amount, currency },
            settled_at,
            fee: fee.map(|f| Money {
                amount_minor_units: f,
                currency: CurrencyCode::new("AED").unwrap(),
            }),
            connector_id: connector_id.to_string(),
            status: item["status"].as_str().unwrap_or("settled").to_string(),
        });
    }

    Ok(records)
}

/// Parse fixed-width settlement file.
fn parse_fixed_width(
    content: &str,
    connector_id: &str,
) -> Result<Vec<SettlementRecord>, PlatformError> {
    let mut records = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.len() < 54 {
            continue;
        }

        let acquirer_reference = line[0..20].trim().to_string();
        let amount = line[20..31].trim().parse::<i64>().unwrap_or(0);
        let currency_str = line[31..34].trim();
        let currency =
            CurrencyCode::new(currency_str).unwrap_or_else(|_| CurrencyCode::new("AED").unwrap());
        let settled_at_str = line[34..54].trim();
        let settled_at = chrono::DateTime::parse_from_rfc3339(settled_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let fee = if line.len() >= 65 {
            line[54..65].trim().parse::<i64>().ok()
        } else {
            None
        };

        records.push(SettlementRecord {
            acquirer_reference,
            amount: Money { amount_minor_units: amount, currency },
            settled_at,
            fee: fee.map(|f| Money {
                amount_minor_units: f,
                currency: CurrencyCode::new("AED").unwrap(),
            }),
            connector_id: connector_id.to_string(),
            status: "settled".to_string(),
        });
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv() {
        let csv = "acquirer_reference,amount,currency,settled_at,fee,status\nNI_001,10000,AED,2024-01-15T10:00:00Z,100,captured\nNI_002,5000,AED,2024-01-15T11:00:00Z,50,captured";
        let records = parse_csv(csv, "ni").unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].acquirer_reference, "NI_001");
        assert_eq!(records[0].amount.amount_minor_units, 10000);
        assert_eq!(records[1].acquirer_reference, "NI_002");
    }

    #[test]
    fn test_parse_json() {
        let json = r#"[{"transaction_id":"NI_001","amount":10000,"currency":"AED","settled_at":"2024-01-15T10:00:00Z","fee":100}]"#;
        let records = parse_json(json, "ni").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].acquirer_reference, "NI_001");
        assert_eq!(records[0].fee.as_ref().unwrap().amount_minor_units, 100);
    }

    #[test]
    fn test_parse_empty_csv() {
        let csv = "acquirer_reference,amount,currency,settled_at,fee\n";
        let records = parse_csv(csv, "ni").unwrap();
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn test_parse_json_empty_array() {
        let json = "[]";
        let records = parse_json(json, "ni").unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn test_parse_csv_skips_malformed() {
        let csv = "acquirer_reference,amount,currency,settled_at,fee\nNI_001,10000,AED,2024-01-15T10:00:00Z,100\nbad_row\nNI_002,5000,AED,2024-01-15T11:00:00Z,50,captured";
        let records = parse_csv(csv, "ni").unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_parse_fixed_width() {
        // Format: 0-19: ref, 20-30: amount, 31-33: currency, 34-53: date, 54-64: fee
        let line = format!(
            "{:<20}{:>11}{:<3}{:<20}{:>11}",
            "NI_001", "10000", "AED", "2024-01-15T10:00:00Z", "100"
        );
        let records = parse_fixed_width(&line, "ni").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].acquirer_reference, "NI_001");
    }
}
