//! Connector base URL value object with SSRF protection per SRS SSRF-001/002.
//!
//! All acquirer API endpoint URLs must be validated against private/reserved
//! IP ranges before registration to prevent SSRF attacks.

use platform_middleware::ssrf::{validate_url_async, SsrfCheckResult};
use platform_error::PlatformError;

/// Validated connector base URL — only contains URLs that have passed SSRF checks.
/// Used for acquirer API endpoints during connector onboarding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorBaseUrl(String);

impl ConnectorBaseUrl {
    /// Create a new connector base URL with async SSRF validation.
    ///
    /// Per SRS SSRF-001: Validates target URL against deny-list of private/reserved IP ranges.
    /// Per SRS SSRF-002: Only HTTPS URLs allowed; DNS resolution checked for rebinding.
    /// Uses async DNS resolution to avoid blocking the Tokio runtime.
    pub async fn new(url: &str) -> Result<Self, PlatformError> {
        match validate_url_async(url).await {
            SsrfCheckResult::Allowed => Ok(Self(url.to_string())),
            SsrfCheckResult::Blocked(reason) => Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField(
                    format!("SSRF blocked: {reason}")
                )
            )),
        }
    }

    /// Get the URL string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner URL string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for ConnectorBaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ConnectorBaseUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_https_url() {
        let result = ConnectorBaseUrl::new("https://api.checkout.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "https://api.checkout.com");
    }

    #[tokio::test]
    async fn test_rejects_http() {
        let result = ConnectorBaseUrl::new("http://api.checkout.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_loopback_ip() {
        let result = ConnectorBaseUrl::new("https://127.0.0.1/api").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_private_ip() {
        let result = ConnectorBaseUrl::new("https://192.168.1.1/api").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_10_x_range() {
        let result = ConnectorBaseUrl::new("https://10.0.0.1/api").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_invalid_url() {
        let result = ConnectorBaseUrl::new("not-a-url").await;
        assert!(result.is_err());
    }
}
