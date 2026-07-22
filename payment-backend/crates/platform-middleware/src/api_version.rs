/// API version extraction from URL path.
pub fn extract_version(path: &str) -> Option<u32> {
    if path.starts_with("/v1/") {
        Some(1)
    } else if path.starts_with("/v2/") {
        Some(2)
    } else {
        None
    }
}
