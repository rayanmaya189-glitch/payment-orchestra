/// Server-Side Request Forgery prevention.
pub fn is_allowed_url(url: &str) -> bool {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            // Block private IP ranges
            if host == "localhost" || host == "127.0.0.1" || host.starts_with("10.") 
                || host.starts_with("172.16.") || host.starts_with("192.168.") 
                || host == "[::1]" {
                return false;
            }
            return parsed.scheme() == "https";
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_localhost() {
        assert!(!is_allowed_url("http://localhost:8080/webhook"));
    }

    #[test]
    fn test_rejects_private_ip() {
        assert!(!is_allowed_url("http://192.168.1.1/secret"));
    }

    #[test]
    fn test_allows_https() {
        assert!(is_allowed_url("https://api.acquirer.com/v1/chargebacks"));
    }

    #[test]
    fn test_rejects_http() {
        assert!(!is_allowed_url("http://example.com"));
    }
}
