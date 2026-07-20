#[derive(Debug, Clone)]
pub struct RateLimitConfig { pub max_requests: u32, pub window_seconds: u32 }
