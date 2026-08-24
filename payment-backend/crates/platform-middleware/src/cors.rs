/// CORS configuration.
pub fn cors_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Access-Control-Allow-Methods", "POST, PATCH, DELETE, OPTIONS"),
        ("Access-Control-Allow-Headers", "Content-Type, Authorization, Idempotency-Key"),
        ("Access-Control-Max-Age", "86400"),
    ]
}
