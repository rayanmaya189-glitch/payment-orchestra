//! Server-Side Request Forgery (SSRF) prevention.
//!
//! Provides URL validation and a [`reqwest::Client`]-like wrapper that
//! automatically rejects requests to private/internal network addresses.
//!
//! # Usage
//!
//! ```rust,ignore
//! use platform_middleware::ssrf::SsrfSafeClient;
//!
//! let client = SsrfSafeClient::new()?;
//! let resp = client.get("https://api.acquirer.com/status").await?; // ✅ allowed
//! let resp = client.get("http://192.168.1.1/admin").await?;        // ❌ rejected
//! ```

use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::OnceLock;

// ─── Cached Environment Check ────────────────────────────────────────────────

/// Whether HTTP URLs are allowed (cached after first read).
fn allow_http_urls() -> bool {
    static ALLOW_HTTP: OnceLock<bool> = OnceLock::new();
    *ALLOW_HTTP.get_or_init(|| std::env::var("SSRF_ALLOW_HTTP").is_ok())
}

// ─── Blocked Hosts ───────────────────────────────────────────────────────────

/// Returns true if the host is a private/loopback/link-local address that
/// should never be reachable from a payment connector.
fn is_blocked_host(host: &str) -> bool {
    // Common hostnames
    if host.eq_ignore_ascii_case("localhost")
        || host.eq_ignore_ascii_case("localhost.localdomain")
        || host.eq_ignore_ascii_case("broadcasthost")
        || host == "[::1]"
    {
        return true;
    }

    // Try parsing as IPv4 address
    if let Ok(ipv4) = host.parse::<Ipv4Addr>() {
        return ipv4.is_loopback()
            || ipv4.is_private()
            || ipv4.is_link_local()
            || ipv4.is_multicast()
            || ipv4.is_unspecified()
            // 127.0.0.0/8 all loopback, not just 127.0.0.1
            || (ipv4.octets()[0] == 127)
            // 169.254.0.0/16 link-local
            || (ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254)
            // 0.0.0.0/8 "this network"
            || (ipv4.octets()[0] == 0);
    }

    // Try parsing as IPv6 address
    if let Ok(ipv6) = host.parse::<Ipv6Addr>() {
        return ipv6.is_loopback()
            || ipv6.is_multicast()
            || ipv6.is_unspecified()
            // Unique local address (fc00::/7)
            || (ipv6.octets()[0] & 0xfe == 0xfc);
    }

    // Check for private-range hostnames that aren't valid IPs
    // (e.g., "10.0.0.0.nip.io", "192.168.1.1.sslip.io")
    if host.starts_with("10.")
        || host.starts_with("172.16.")
        || host.starts_with("172.17.")
        || host.starts_with("172.18.")
        || host.starts_with("172.19.")
        || host.starts_with("172.20.")
        || host.starts_with("172.21.")
        || host.starts_with("172.22.")
        || host.starts_with("172.23.")
        || host.starts_with("172.24.")
        || host.starts_with("172.25.")
        || host.starts_with("172.26.")
        || host.starts_with("172.27.")
        || host.starts_with("172.28.")
        || host.starts_with("172.29.")
        || host.starts_with("172.30.")
        || host.starts_with("172.31.")
        || host.starts_with("192.168.")
        || host.starts_with("0.")
    {
        return true;
    }

    false
}

// ─── URL Validation (from parsed Url) ────────────────────────────────────────

/// Validate a pre-parsed `url::Url` against SSRF rules.
///
/// This is the zero-copy variant — use it when you already have a parsed
/// `url::Url` to avoid parsing the URL string twice.
pub fn validate_parsed_url(parsed: &url::Url) -> Result<(), SsrfError> {
    // Enforce HTTPS for all external calls unless explicitly overridden
    if parsed.scheme() != "https" && !allow_http_urls() {
        return Err(SsrfError::InsecureScheme(parsed.scheme().into()));
    }

    // Check the host against blocked ranges
    if let Some(host) = parsed.host_str() {
        if is_blocked_host(host) {
            return Err(SsrfError::BlockedHost(host.into()));
        }
    } else {
        return Err(SsrfError::MalformedUrl("no host in URL".into()));
    }

    Ok(())
}

// ─── URL Validation (from string) ────────────────────────────────────────────

/// Parse and validate that a URL string is safe to call from a connector.
///
/// Returns an error (with a descriptive message) if the URL:
/// - Points to a private, loopback, or link-local IP address
/// - Uses plain HTTP (not HTTPS), unless overridden via `SSRF_ALLOW_HTTP`
/// - Is malformed / unparseable
pub fn validate_connector_url(url: &str) -> Result<(), SsrfError> {
    let parsed =
        url::Url::parse(url).map_err(|e| SsrfError::MalformedUrl(e.to_string()))?;
    validate_parsed_url(&parsed)
}

/// Convenience wrapper — returns true if the URL is safe to call.
pub fn is_allowed_url(url: &str) -> bool {
    validate_connector_url(url).is_ok()
}

// ─── Error Type ──────────────────────────────────────────────────────────────

/// Errors that can occur during SSRF validation or request execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsrfError {
    /// URL could not be parsed.
    MalformedUrl(String),
    /// URL uses HTTP instead of HTTPS.
    InsecureScheme(String),
    /// URL points to a blocked (private/loopback) host.
    BlockedHost(String),
    /// Transport error during request execution.
    TransportError(String),
}

impl std::fmt::Display for SsrfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SsrfError::MalformedUrl(detail) => write!(f, "Malformed URL: {detail}"),
            SsrfError::InsecureScheme(scheme) => {
                write!(f, "Insecure URL scheme '{scheme}': HTTPS required")
            }
            SsrfError::BlockedHost(host) => {
                write!(f, "Blocked host '{host}': private/internal address not allowed")
            }
            SsrfError::TransportError(detail) => write!(f, "Request failed: {detail}"),
        }
    }
}

impl std::error::Error for SsrfError {}

// ─── SSRF-Safe HTTP Client ───────────────────────────────────────────────────

/// A [`reqwest::Client`]-like wrapper that validates every outbound URL
/// against SSRF rules before making the request.
///
/// Use this in connector implementations instead of a raw `reqwest::Client`
/// to prevent server-side request forgery attacks.
#[derive(Debug, Clone)]
pub struct SsrfSafeClient {
    inner: reqwest::Client,
}

impl SsrfSafeClient {
    /// Create a new SSRF-safe HTTP client with default reqwest settings.
    pub fn new() -> Result<Self, reqwest::Error> {
        let inner = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("PaymentOrchestra-SSRF/1.0")
            .build()?;
        Ok(Self { inner })
    }

    /// Create from an existing reqwest `Client`.
    pub fn from_client(client: reqwest::Client) -> Self {
        Self { inner: client }
    }

    /// Get a reference to the inner reqwest client (for advanced use).
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }

    /// Validate a URL string without double-parsing.
    /// Returns the parsed `url::Url` so callers can avoid a second parse.
    pub fn validate_url(url: &str) -> Result<url::Url, SsrfError> {
        let parsed =
            url::Url::parse(url).map_err(|e| SsrfError::MalformedUrl(e.to_string()))?;
        validate_parsed_url(&parsed)?;
        Ok(parsed)
    }

    /// Perform a validated GET request.
    pub async fn get(&self, url: &str) -> Result<reqwest::Response, SsrfError> {
        let validated = Self::validate_url(url)?;
        self.inner
            .get(validated.as_str())
            .send()
            .await
            .map_err(|e| SsrfError::TransportError(e.to_string()))
    }

    /// Perform a validated POST request.
    pub async fn post(&self, url: &str) -> Result<reqwest::Response, SsrfError> {
        let validated = Self::validate_url(url)?;
        self.inner
            .post(validated.as_str())
            .send()
            .await
            .map_err(|e| SsrfError::TransportError(e.to_string()))
    }

    /// Create a validated GET request builder (for adding headers, body, etc.).
    pub fn get_with(&self, url: &str) -> Result<reqwest::RequestBuilder, SsrfError> {
        let validated = Self::validate_url(url)?;
        Ok(self.inner.get(validated.as_str()))
    }

    /// Create a validated POST request builder.
    pub fn post_with(&self, url: &str) -> Result<reqwest::RequestBuilder, SsrfError> {
        let validated = Self::validate_url(url)?;
        Ok(self.inner.post(validated.as_str()))
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_localhost() {
        assert!(!is_allowed_url("http://localhost:8080/webhook"));
        // Scheme check fires before host check for http:// URLs
        assert_eq!(
            validate_connector_url("http://localhost:8080/webhook"),
            Err(SsrfError::InsecureScheme("http".into()))
        );
        // HTTPS on loopback should be blocked by host check
        assert_eq!(
            validate_connector_url("https://localhost:8080/webhook"),
            Err(SsrfError::BlockedHost("localhost".into()))
        );
    }

    #[test]
    fn test_rejects_private_ip() {
        assert!(!is_allowed_url("http://192.168.1.1/secret"));
        assert!(!is_allowed_url("https://10.0.0.1/admin"));
        assert!(!is_allowed_url("https://172.16.0.1/v1/payments"));
    }

    #[test]
    fn test_rejects_ipv6_loopback() {
        assert!(!is_allowed_url("http://[::1]:9001/debug"));
    }

    #[test]
    fn test_allows_https_production() {
        assert!(is_allowed_url("https://api.stripe.com/v1/charges"));
        assert!(is_allowed_url("https://api.acquirer.com/v1/chargebacks"));
    }

    #[test]
    fn test_rejects_http_by_default() {
        assert!(!is_allowed_url("http://example.com/data"));
        assert!(!is_allowed_url("http://api.stripe.com/v1/charges"));
    }

    #[test]
    fn test_rejects_malformed_url() {
        assert!(!is_allowed_url("not a url"));
        assert!(matches!(
            validate_connector_url("not a url"),
            Err(SsrfError::MalformedUrl(_))
        ));
    }

    #[test]
    fn test_rejects_loopback_ip() {
        assert!(!is_allowed_url("https://127.0.0.1:8080/health"));
        assert!(!is_allowed_url("https://127.0.1.1:8080/health"));
        assert!(!is_allowed_url("https://127.64.0.1:8080/health"));
    }

    #[test]
    fn test_allows_https_with_path() {
        assert!(is_allowed_url(
            "https://checkout.com/api/v1/payments?test=true"
        ));
    }

    #[test]
    fn test_rejects_link_local() {
        assert!(!is_allowed_url("https://169.254.169.254/latest/meta-data/"));
    }

    #[test]
    fn test_rejects_multicast() {
        assert!(!is_allowed_url("https://224.0.0.1/test"));
    }

    #[test]
    fn test_rejects_unspecified() {
        assert!(!is_allowed_url("https://0.0.0.0/test"));
    }

    #[test]
    fn test_rejects_ula_ipv6() {
        assert!(!is_allowed_url("https://fd00::1/test"));
    }

    #[test]
    fn test_ssrf_safe_client_get_validates() {
        let result = SsrfSafeClient::validate_url("https://api.stripe.com/v1/charges");
        assert!(result.is_ok());

        // HTTPS private IP is blocked by host check
        let result = SsrfSafeClient::validate_url("https://192.168.1.1/secret");
        assert!(result.is_err());
        assert_eq!(
            result,
            Err(SsrfError::BlockedHost("192.168.1.1".into()))
        );

        // HTTP URL is blocked by scheme check first
        let result = SsrfSafeClient::validate_url("http://192.168.1.1/secret");
        assert!(result.is_err());
        assert_eq!(result, Err(SsrfError::InsecureScheme("http".into())));
    }

    #[test]
    fn test_ssrf_safe_client_rejects_http() {
        let result = SsrfSafeClient::validate_url("http://example.com/data");
        assert!(result.is_err());
        assert_eq!(result, Err(SsrfError::InsecureScheme("http".into())));
    }

    #[test]
    fn test_allows_nip_io_with_public_ip() {
        assert!(is_allowed_url("https://api.example.com/v1/payments"));
    }

    #[test]
    fn test_rejects_hostname_starting_with_private_prefix() {
        assert!(!is_allowed_url("https://192.168.1.1.sslip.io/test"));
        assert!(!is_allowed_url("https://10.0.0.1.nip.io/test"));
    }

    #[test]
    fn test_display_error() {
        let err = SsrfError::BlockedHost("127.0.0.1".into());
        assert_eq!(
            err.to_string(),
            "Blocked host '127.0.0.1': private/internal address not allowed"
        );

        let err = SsrfError::TransportError("connection refused".into());
        assert_eq!(err.to_string(), "Request failed: connection refused");
    }

    #[test]
    fn test_validate_parsed_url_no_double_parse() {
        // Verify validate_parsed_url works correctly (no re-parsing needed)
        let parsed = url::Url::parse("https://api.stripe.com/v1/charges").unwrap();
        assert!(validate_parsed_url(&parsed).is_ok());

        let parsed = url::Url::parse("https://192.168.1.1/secret").unwrap();
        assert_eq!(
            validate_parsed_url(&parsed),
            Err(SsrfError::BlockedHost("192.168.1.1".into()))
        );
    }
}
