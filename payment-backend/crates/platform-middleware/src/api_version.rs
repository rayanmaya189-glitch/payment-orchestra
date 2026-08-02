//! API versioning middleware — version extraction, negotiation, and deprecation.
//!
//! Supports:
//! - URL path versioning (/v1/, /v2/)
//! - Header-based versioning (Accept-Version, X-API-Version)
//! - Version negotiation
//! - Deprecation warnings for old versions
//! - Sunset headers for deprecated versions

use http::{Request, Response, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

// ─── Version Constants ───────────────────────────────────────────────────────

/// Current stable API version.
pub const CURRENT_VERSION: u32 = 1;

/// Supported API versions.
pub const SUPPORTED_VERSIONS: &[u32] = &[1];

/// Deprecated versions (still functional but will be removed).
pub const DEPRECATED_VERSIONS: &[u32] = &[];

/// Sunset date for deprecated versions (RFC 8594).
pub const SUNSET_DATE: &str = "2027-01-01";

// ─── Version Extraction ──────────────────────────────────────────────────────

/// Extract API version from request.
///
/// Precedence:
/// 1. URL path (/v1/..., /v2/...)
/// 2. Accept-Version header
/// 3. X-API-Version header
/// 4. Default to current version
pub fn extract_version_from_request<B>(req: &Request<B>) -> ApiVersion {
    // 1. Try URL path
    if let Some(version) = extract_version_from_path(req.uri().path()) {
        return version;
    }

    // 2. Try Accept-Version header
    if let Some(version) = req
        .headers()
        .get("accept-version")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        return validate_version(version);
    }

    // 3. Try X-API-Version header
    if let Some(version) = req
        .headers()
        .get("x-api-version")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        return validate_version(version);
    }

    // 4. Default to current version
    ApiVersion {
        major: CURRENT_VERSION,
        is_deprecated: false,
    }
}

/// Extract version from URL path.
fn extract_version_from_path(path: &str) -> Option<ApiVersion> {
    let path = path.trim_start_matches('/');
    if let Some(rest) = path.strip_prefix('v') {
        if let Some(version_str) = rest.split('/').next() {
            if let Ok(version) = version_str.parse::<u32>() {
                return Some(validate_version(version));
            }
        }
    }
    None
}

/// Validate and wrap a version number.
fn validate_version(version: u32) -> ApiVersion {
    ApiVersion {
        major: version,
        is_deprecated: DEPRECATED_VERSIONS.contains(&version),
    }
}

// ─── Version Types ───────────────────────────────────────────────────────────

/// API version information.
#[derive(Debug, Clone)]
pub struct ApiVersion {
    pub major: u32,
    pub is_deprecated: bool,
}

impl ApiVersion {
    /// Check if this version is supported.
    pub fn is_supported(&self) -> bool {
        SUPPORTED_VERSIONS.contains(&self.major)
    }

    /// Check if this version is current.
    pub fn is_current(&self) -> bool {
        self.major == CURRENT_VERSION
    }
}

// ─── Version Extension ───────────────────────────────────────────────────────

/// Extension key for API version in request/response extensions.
#[derive(Debug, Clone)]
pub struct ApiVersionExtension(pub ApiVersion);

// ─── Version Layer ───────────────────────────────────────────────────────────

/// Layer that extracts API version and adds version headers to responses.
#[derive(Clone)]
pub struct ApiVersionLayer;

impl<S> Layer<S> for ApiVersionLayer {
    type Service = ApiVersionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ApiVersionService { inner }
    }
}

// ─── Version Service ─────────────────────────────────────────────────────────

/// Service that extracts API version and adds version headers.
#[derive(Clone)]
pub struct ApiVersionService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for ApiVersionService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let version = extract_version_from_request(&req);

        // Add version to request extensions
        let mut req = req;
        req.extensions_mut()
            .insert(ApiVersionExtension(version.clone()));

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut response = inner.call(req).await?;

            // Add version headers to response
            let headers = response.headers_mut();

            // X-API-Version: current version being used
            headers.insert(
                "x-api-version",
                version.major.to_string().parse().unwrap(),
            );

            // X-API-Current-Version: the latest stable version
            headers.insert(
                "x-api-current-version",
                CURRENT_VERSION.to_string().parse().unwrap(),
            );

            // Deprecation warning if using deprecated version
            if version.is_deprecated {
                headers.insert("deprecation", "true".parse().unwrap());
                headers.insert(
                    "sunset",
                    format!("date=\"{}\"", SUNSET_DATE).parse().unwrap(),
                );
                headers.insert(
                    "x-api-deprecation-notice",
                    format!(
                        "API version {} is deprecated. Please migrate to version {}. Sunset date: {}",
                        version.major, CURRENT_VERSION, SUNSET_DATE
                    )
                    .parse()
                    .unwrap(),
                );
            }

            // X-API-Supported-Versions: list all supported versions
            headers.insert(
                "x-api-supported-versions",
                SUPPORTED_VERSIONS
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
                    .parse()
                    .unwrap(),
            );

            Ok(response)
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_path() {
        let version = extract_version_from_path("/v1/payment-intents");
        assert!(version.is_some());
        assert_eq!(version.unwrap().major, 1);
    }

    #[test]
    fn test_extract_version_from_path_v2() {
        let version = extract_version_from_path("/v2/connectors");
        assert!(version.is_some());
        assert_eq!(version.unwrap().major, 2);
    }

    #[test]
    fn test_extract_version_no_version() {
        let version = extract_version_from_path("/health");
        assert!(version.is_none());
    }

    #[test]
    fn test_validate_version() {
        let v1 = validate_version(1);
        assert!(v1.is_supported());
        assert!(v1.is_current());

        let v99 = validate_version(99);
        assert!(!v99.is_supported());
        assert!(!v99.is_current());
    }

    #[test]
    fn test_api_version_properties() {
        let current = ApiVersion {
            major: 1,
            is_deprecated: false,
        };
        assert!(current.is_supported());
        assert!(current.is_current());
        assert!(!current.is_deprecated);

        let deprecated = ApiVersion {
            major: 0,
            is_deprecated: true,
        };
        assert!(!deprecated.is_supported());
        assert!(!deprecated.is_current());
        assert!(deprecated.is_deprecated);
    }
}
