use std::fmt;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u32,
}

impl RateLimitConfig {
    pub fn new(max_requests: u32, window_seconds: u32) -> Result<Self, String> {
        if max_requests == 0 {
            return Err("max_requests must be > 0".into());
        }
        if window_seconds == 0 {
            return Err("window_seconds must be > 0".into());
        }
        Ok(Self {
            max_requests,
            window_seconds,
        })
    }

    pub fn default_api() -> Self {
        Self {
            max_requests: 1000,
            window_seconds: 60,
        }
    }

    pub fn default_auth() -> Self {
        Self {
            max_requests: 30,
            window_seconds: 60,
        }
    }
}

impl fmt::Display for RateLimitConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} req/{}s", self.max_requests, self.window_seconds)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            "PATCH" => Some(Self::Patch),
            "HEAD" => Some(Self::Head),
            "OPTIONS" => Some(Self::Options),
            _ => None,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathPattern {
    pub pattern: String,
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone)]
pub enum PathSegment {
    Static(String),
    Param(String),
    Wildcard,
}

impl PathPattern {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let segments: Vec<PathSegment> = pattern
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/')
            .map(|s| {
                if s.starts_with('{') && s.ends_with('}') {
                    PathSegment::Param(s[1..s.len() - 1].to_string())
                } else if s == "*" {
                    PathSegment::Wildcard
                } else {
                    PathSegment::Static(s.to_string())
                }
            })
            .collect();

        Ok(Self {
            pattern: pattern.to_string(),
            segments,
        })
    }

    pub fn matches(&self, path: &str) -> bool {
        let path_segments: Vec<&str> = path
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        self.matches_segments(&path_segments)
    }

    fn matches_segments(&self, path_segments: &[&str]) -> bool {
        let mut pattern_idx = 0;
        let mut path_idx = 0;

        while pattern_idx < self.segments.len() {
            match &self.segments[pattern_idx] {
                PathSegment::Wildcard => return true,
                PathSegment::Param(_) => {
                    if path_idx >= path_segments.len() {
                        return false;
                    }
                    pattern_idx += 1;
                    path_idx += 1;
                }
                PathSegment::Static(s) => {
                    if path_idx >= path_segments.len() {
                        return false;
                    }
                    if path_segments[path_idx] != s.as_str() {
                        return false;
                    }
                    pattern_idx += 1;
                    path_idx += 1;
                }
            }
        }

        path_idx == path_segments.len()
    }

    pub fn is_prefix_match(&self, path: &str) -> bool {
        let path_segments: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if path_segments.len() < self.segments.len() {
            return false;
        }

        for (i, segment) in self.segments.iter().enumerate() {
            match segment {
                PathSegment::Static(s) => {
                    if path_segments[i] != s.as_str() {
                        return false;
                    }
                }
                PathSegment::Wildcard => return true,
                PathSegment::Param(_) => continue,
            }
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardStatus {
    Success,
    Timeout,
    ConnectionRefused,
    UpstreamError,
    RateLimited,
}

impl ForwardStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Success => "success",
            Self::Timeout => "timeout",
            Self::ConnectionRefused => "connection_refused",
            Self::UpstreamError => "upstream_error",
            Self::RateLimited => "rate_limited",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::new(100, 60).unwrap();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_seconds, 60);
        assert!(RateLimitConfig::new(0, 60).is_err());
        assert!(RateLimitConfig::new(100, 0).is_err());
    }

    #[test]
    fn test_http_method() {
        assert_eq!(HttpMethod::from_str("get"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("POST"), Some(HttpMethod::Post));
        assert_eq!(HttpMethod::from_str("INVALID"), None);
    }

    #[test]
    fn test_path_pattern_static() {
        let pattern = PathPattern::parse("/v1/payments").unwrap();
        assert!(pattern.matches("/v1/payments"));
        assert!(!pattern.matches("/v1/payments/123"));
        assert!(!pattern.matches("/v1/users"));
    }

    #[test]
    fn test_path_pattern_param() {
        let pattern = PathPattern::parse("/v1/payments/{id}").unwrap();
        assert!(pattern.matches("/v1/payments/abc-123"));
        assert!(pattern.matches("/v1/payments/123"));
        assert!(!pattern.matches("/v1/payments"));
        assert!(!pattern.matches("/v1/payments/123/extra"));
    }

    #[test]
    fn test_path_pattern_wildcard() {
        let pattern = PathPattern::parse("/v1/*").unwrap();
        assert!(pattern.matches("/v1/payments"));
        assert!(pattern.matches("/v1/payments/123"));
        assert!(pattern.matches("/v1/anything/at/all"));
    }

    #[test]
    fn test_prefix_match() {
        let pattern = PathPattern::parse("/v1/payments").unwrap();
        assert!(pattern.is_prefix_match("/v1/payments"));
        assert!(pattern.is_prefix_match("/v1/payments/123"));
        assert!(!pattern.is_prefix_match("/v1/users"));
    }
}
