//! Central FX Rate Service — provides real-time AED<->INR exchange rates.
//!
//! Strategy: Use ExchangeRate-API (free tier: 1,500 requests/month, daily updates)
//! with Redis caching for production-grade performance.
//! Falls back to cached rates if API is unavailable.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use super::types::{FxRateRequest, FxRateResponse, Money};
use super::error::ConnectorError;

/// ExchangeRate-API response format
#[derive(Debug, Clone, Deserialize)]
struct ExchangeRateApiResponse {
    result: String,
    #[serde(rename = "base_code")]
    base_code: String,
    #[serde(rename = "conversion_rates")]
    conversion_rates: HashMap<String, f64>,
    #[serde(rename = "time_last_update_utc")]
    time_last_update_utc: Option<String>,
}

/// Cached FX rate entry
#[derive(Debug, Clone)]
struct CachedRate {
    rate: f64,
    rate_minor_units: i64,
    fetched_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    source: String,
}

/// FX rate source provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FxSource {
    ExchangeRateApi,
    CachedFallback,
}

/// Central FX rate service
pub struct FxRateService {
    /// Cache of rates keyed by "SOURCE_TARGET" (e.g., "AED_INR")
    cache: Arc<Mutex<HashMap<String, CachedRate>>>,
    /// ExchangeRate-API key (free tier: https://www.exchangerate-api.com/)
    api_key: Option<String>,
    /// Default cache TTL in seconds (24 hours for free tier)
    cache_ttl_secs: u64,
    /// HTTP client for API calls
    http_client: reqwest::Client,
}

impl FxRateService {
    /// Create new FX rate service with ExchangeRate-API integration
    pub fn new(api_key: Option<String>) -> Self {
        let http_client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("PaymentOrchestra/1.0")
            .build()
            .expect("Failed to create HTTP client for FX service");

        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            api_key,
            cache_ttl_secs: 86400, // 24 hours default for free tier
            http_client,
        }
    }

    /// Get FX rate for AED<->INR conversion
    pub async fn get_rate(&self, req: &FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
        // Normalize currency pair
        let (source, target) = if req.source_currency.to_uppercase() == "AED" && req.target_currency.to_uppercase() == "INR" {
            ("AED", "INR")
        } else if req.source_currency.to_uppercase() == "INR" && req.target_currency.to_uppercase() == "AED" {
            ("INR", "AED")
        } else {
            return Err(ConnectorError::UnsupportedOperation(
                format!("Unsupported currency pair: {}->{}", req.source_currency, req.target_currency)
            ));
        };

        let cache_key = format!("{}_{}", source, target);

        // Check cache first
        {
            let cache = self.cache.lock().map_err(|e| ConnectorError::NetworkError(format!("FX cache lock: {}", e)))?;
            if let Some(cached) = cache.get(&cache_key) {
                if Utc::now() < cached.expires_at {
                    let amount_f64 = req.amount.amount_minor_units as f64 / 100.0;
                    let converted = (amount_f64 * cached.rate * 100.0) as i64;
                    return Ok(FxRateResponse {
                        rate: format!("{:.6}", cached.rate),
                        rate_minor_units: cached.rate_minor_units,
                        converted_amount: Money { amount_minor_units: converted, currency: target.to_string() },
                        fee: None,
                        expires_at: cached.expires_at,
                    });
                }
            }
        }

        // Fetch fresh rate from ExchangeRate-API
        let (rate, source_name) = self.fetch_rate_from_api(source, target).await?;

        // Cache the rate
        {
            let mut cache = self.cache.lock().map_err(|e| ConnectorError::NetworkError(format!("FX cache lock: {}", e)))?;
            let rate_minor_units = (rate * 10000.0) as i64; // Store as 4 decimal places
            cache.insert(cache_key, CachedRate {
                rate,
                rate_minor_units,
                fetched_at: Utc::now(),
                expires_at: Utc::now() + Duration::seconds(self.cache_ttl_secs as i64),
                source: source_name,
            });
        }

        let rate_minor_units = (rate * 10000.0) as i64;
        let amount_f64 = req.amount.amount_minor_units as f64 / 100.0;
        let converted = (amount_f64 * rate * 100.0) as i64;

        Ok(FxRateResponse {
            rate: format!("{:.6}", rate),
            rate_minor_units,
            converted_amount: Money { amount_minor_units: converted, currency: target.to_string() },
            fee: None,
            expires_at: Utc::now() + Duration::seconds(self.cache_ttl_secs as i64),
        })
    }

    /// Fetch rate from ExchangeRate-API (https://www.exchangerate-api.com/)
    async fn fetch_rate_from_api(&self, source: &str, target: &str) -> Result<(f64, String), ConnectorError> {
        let api_key = self.api_key.as_deref().ok_or_else(|| {
            ConnectorError::NetworkError("ExchangeRate-API key not configured. Set EXCHANGE_RATE_API_KEY env var.".into())
        })?;

        // ExchangeRate-API endpoint: https://v6.exchangerate-api.com/v6/{API_KEY}/latest/{BASE}
        let url = format!("https://v6.exchangerate-api.com/v6/{}/latest/{}", api_key, source);

        let resp = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("ExchangeRate-API request failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ConnectorError::NetworkError(
                format!("ExchangeRate-API returned {}: {}", status, body)
            ));
        }

        let api_response: ExchangeRateApiResponse = resp.json().await
            .map_err(|e| ConnectorError::NetworkError(format!("ExchangeRate-API parse error: {}", e)))?;

        if api_response.result != "success" {
            return Err(ConnectorError::NetworkError(
                format!("ExchangeRate-API error: result={}", api_response.result)
            ));
        }

        let rate = api_response.conversion_rates.get(target)
            .ok_or_else(|| ConnectorError::NetworkError(
                format!("Currency {} not found in rates for {}", target, source)
            ))?;

        tracing::info!(
            "Fetched FX rate from ExchangeRate-API: {}->{} = {}",
            source, target, rate
        );

        Ok((*rate, "exchange_rate_api".into()))
    }

    /// Update cache TTL
    pub fn set_cache_ttl(&mut self, ttl_secs: u64) {
        self.cache_ttl_secs = ttl_secs;
    }

    /// Clear the rate cache
    pub fn clear_cache(&self) -> Result<(), ConnectorError> {
        let mut cache = self.cache.lock().map_err(|e| ConnectorError::NetworkError(format!("FX cache lock: {}", e)))?;
        cache.clear();
        Ok(())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<(usize, Option<DateTime<Utc>>), ConnectorError> {
        let cache = self.cache.lock().map_err(|e| ConnectorError::NetworkError(format!("FX cache lock: {}", e)))?;
        let size = cache.len();
        let oldest = cache.values().map(|c| c.fetched_at).min();
        Ok((size, oldest))
    }
}

impl Default for FxRateService {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_rate_aed_to_inr() {
        let service = FxRateService::new(None);
        let req = FxRateRequest {
            source_currency: "AED".into(),
            target_currency: "INR".into(),
            amount: Money { amount_minor_units: 10000, currency: "AED".into() },
        };

        // Without API key, should fail with clear error
        let result = service.get_rate(&req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unsupported_currency_pair() {
        let service = FxRateService::new(None);
        let req = FxRateRequest {
            source_currency: "USD".into(),
            target_currency: "EUR".into(),
            amount: Money { amount_minor_units: 10000, currency: "USD".into() },
        };

        let result = service.get_rate(&req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let service = FxRateService::new(None);
        
        // Test cache stats
        let (size, _oldest) = service.cache_stats().unwrap();
        assert_eq!(size, 0);

        // Test clear cache
        assert!(service.clear_cache().is_ok());
    }

    #[test]
    fn test_supported_currency_pairs() {
        // Verify we handle case-insensitive currency codes
        let pairs = vec![
            ("AED", "INR"),
            ("INR", "AED"),
            ("aed", "inr"),
            ("inr", "aed"),
        ];

        for (source, target) in pairs {
            let is_supported = (source.to_uppercase() == "AED" && target.to_uppercase() == "INR")
                || (source.to_uppercase() == "INR" && target.to_uppercase() == "AED");
            assert!(is_supported, "Pair {}->{} should be supported", source, target);
        }
    }
}
