//! SSRF protection per SRS SSRF-001/002.
//!
//! Validates outbound URLs against private/reserved IP ranges before making requests.
//! Prevents DNS rebinding by resolving DNS and checking resolved IPs.

use std::net::{IpAddr, ToSocketAddrs};

/// RFC 1918 private ranges + other reserved ranges that must not be targeted.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            // 10.0.0.0/8
            if octets[0] == 10 { return true; }
            // 172.16.0.0/12
            if octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31) { return true; }
            // 192.168.0.0/16
            if octets[0] == 192 && octets[1] == 168 { return true; }
            // 127.0.0.0/8 (loopback)
            if octets[0] == 127 { return true; }
            // 169.254.0.0/16 (link-local)
            if octets[0] == 169 && octets[1] == 254 { return true; }
            // 0.0.0.0/8
            if octets[0] == 0 { return true; }
            // Multicast 224.0.0.0/4
            if octets[0] >= 224 && octets[0] <= 239 { return true; }
            false
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() { return true; }
            if v6.is_unicast_link_local() { return true; }
            if v6.is_unique_local() { return true; }
            false
        }
    }
}

/// Result of SSRF validation.
#[derive(Debug)]
pub enum SsrfCheckResult {
    Allowed,
    Blocked(String),
}

/// Validate a URL for SSRF safety per SRS SSRF-001/002.
pub fn validate_url(url_str: &str) -> SsrfCheckResult {
    let url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(e) => return SsrfCheckResult::Blocked(format!("Invalid URL: {e}")),
    };

    // SRS SSRF-002: Only HTTPS allowed
    if url.scheme() != "https" {
        return SsrfCheckResult::Blocked(format!("URL scheme must be HTTPS, got: {}", url.scheme()));
    }

    let host = match url.host_str() {
        Some(h) => h,
        None => return SsrfCheckResult::Blocked("URL has no host".to_string()),
    };

    // Check if host is an IP address directly
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return SsrfCheckResult::Blocked(format!("Host IP {} is in a private/reserved range", ip));
        }
        return SsrfCheckResult::Allowed;
    }

    // Resolve DNS and check resolved IPs (prevents DNS rebinding)
    let lookup = format!("{}:443", host);
    match lookup.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                let ip = addr.ip();
                if is_private_ip(&ip) {
                    return SsrfCheckResult::Blocked(format!(
                        "DNS resolved to private IP {} for host {}", ip, host
                    ));
                }
            }
            SsrfCheckResult::Allowed
        }
        Err(e) => SsrfCheckResult::Blocked(format!("DNS resolution failed: {e}")),
    }
}
