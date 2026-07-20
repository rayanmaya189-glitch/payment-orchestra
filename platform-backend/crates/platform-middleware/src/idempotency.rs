/// Idempotency enforcement using Redis fast-path.
///
/// SRS IDP-001: All mutating commands must include an IdempotencyKey.
/// Redis stores the key with the response for 24 hours.
use redis::aio::ConnectionManager;
use redis::cmd;
use platform_error::PlatformError;

/// Check if an idempotency key has already been processed.
///
/// Returns Some(response) if already processed, None if new.
pub async fn check_idempotency(
    redis: &mut ConnectionManager,
    key: &str,
) -> Result<Option<String>, PlatformError> {
    let result: Option<String> = cmd("GET")
        .arg(format!("idempotency:{}", key))
        .query_async(redis)
        .await
        .map_err(|e| PlatformError::Internal(format!("Redis GET failed: {e}")))?;

    Ok(result)
}

/// Store an idempotency key with the response.
///
/// TTL is 24 hours (86400 seconds).
pub async fn store_idempotency(
    redis: &mut ConnectionManager,
    key: &str,
    response: &str,
) -> Result<(), PlatformError> {
    cmd("SET")
        .arg(format!("idempotency:{}", key))
        .arg(response)
        .arg("EX")
        .arg(86400)
        .query_async::<()>(&mut *redis)
        .await
        .map_err(|e| PlatformError::Internal(format!("Redis SET failed: {e}")))?;

    Ok(())
}

/// Try to claim an idempotency key atomically.
///
/// Returns true if this caller claimed the key (first request).
/// Returns false if another request already claimed it.
pub async fn claim_idempotency(
    redis: &mut ConnectionManager,
    key: &str,
) -> Result<bool, PlatformError> {
    let result: u32 = cmd("SET")
        .arg(format!("idempotency:{}", key))
        .arg("processing")
        .arg("NX")
        .arg("EX")
        .arg(300) // 5 minute lock for processing
        .query_async(&mut *redis)
        .await
        .map_err(|e| PlatformError::Internal(format!("Redis SET NX failed: {e}")))?;

    Ok(result == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotency_key_format() {
        let key = "test_key_123";
        assert_eq!(format!("idempotency:{}", key), "idempotency:test_key_123");
    }
}
