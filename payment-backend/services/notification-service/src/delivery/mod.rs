//! Webhook Delivery Engine — at-least-once HTTP delivery for webhook notifications.
//!
//! ## Architecture
//!
//! The delivery engine runs as a background task in the notification service.
//! It polls the repository for pending (Queued/Failed) notification requests
//! and delivers them via HTTPS POST with HMAC-SHA256 signature.
//!
//! ## Delivery Flow
//!
//! 1. Poll for pending notifications every `POLL_INTERVAL_MS`
//! 2. For each pending notification:
//!    a. If channel is Webhook, look up the webhook registration
//!    b. Build HMAC-SHA256 signature from payload using webhook secret
//!    c. POST to webhook URL with signature in `X-Webhook-Signature` header
//!    d. On 2xx response: mark as Sent
//!    e. On non-2xx or error: mark as Failed (retry up to max_retries)
//! 3. Log all delivery attempts with metrics
//!
//! ## Retry Strategy
//!
//! Failed deliveries are retried with exponential backoff:
//! - Retry 1: 30 seconds
//! - Retry 2: 5 minutes
//! - Retry 3: 30 minutes
//! - Max retries: 3 (configurable)

use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};

use platform_middleware::ssrf::SsrfSafeClient;
use crate::repository::{NotificationRepository, WebhookRepository};

mod webhook_delivery;
pub(crate) use webhook_delivery::*;

/// Default polling interval for the delivery queue.
pub const POLL_INTERVAL_MS: u64 = 5_000; // 5 seconds

/// Maximum number of concurrent delivery attempts.
pub const MAX_CONCURRENT_DELIVERIES: usize = 10;

/// Retry delays in milliseconds for successive attempts.
pub const RETRY_DELAYS_MS: &[u64] = &[30_000, 300_000, 1_800_000]; // 30s, 5min, 30min

/// Start the webhook delivery engine as a background task.
///
/// This function spawns a long-running task that polls the notification
/// repository for pending webhook notifications and delivers them.
pub async fn start_delivery_engine(
    notification_repo: Arc<dyn NotificationRepository>,
    webhook_repo: Arc<dyn WebhookRepository>,
    event_bus: Arc<dyn platform_messaging::event_bus::EventBus>,
    max_concurrent: usize,
) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let mut interval = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));

    // Build an SSRF-safe HTTP client for outbound webhook deliveries
    let http_client = match SsrfSafeClient::new() {
        Ok(client) => client,
        Err(e) => {
            error!(error = %e, "Failed to create SSRF-safe HTTP client for webhook delivery");
            return;
        }
    };
    let http_client = Arc::new(http_client);

    info!(
        poll_interval_ms = POLL_INTERVAL_MS,
        max_concurrent = max_concurrent,
        "Webhook delivery engine started"
    );

    loop {
        interval.tick().await;

        // Fetch pending notifications (Queued or Failed)
        let pending = match notification_repo.find_pending().await {
            Ok(pending) => pending,
            Err(e) => {
                error!(error = %e, "Failed to fetch pending notifications");
                continue;
            }
        };

        if pending.is_empty() {
            continue;
        }

        // Process each pending notification concurrently (bounded by semaphore)
        for notification in pending {
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    warn!("Delivery semaphore closed, skipping batch");
                    break;
                }
            };

            let repo = notification_repo.clone();
            let wh_repo = webhook_repo.clone();
            let client = http_client.clone();
            let eb = event_bus.clone();

            tokio::spawn(async move {
                let result = deliver_notification(
                    &*repo,
                    &*wh_repo,
                    &client,
                    &eb,
                    notification,
                ).await;

                if let Err(e) = result {
                    warn!(error = %e, "Webhook delivery attempt completed with error");
                }

                drop(permit);
            });
        }
    }
}
