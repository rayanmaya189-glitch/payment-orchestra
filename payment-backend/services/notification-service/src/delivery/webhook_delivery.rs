//! Webhook delivery logic — HMAC-SHA256 signing, HTTP POST, status tracking.
//!
//! Each notification is delivered to the registered webhook URL with:
//! - `Content-Type: application/json`
//! - `X-Webhook-Signature: sha256=<hex-encoded-HMAC>`
//! - `X-Webhook-ID: <webhook-uuid>`
//! - `X-Webhook-Timestamp: <unix-ms>`
//! - `User-Agent: PaymentOrchestra-Webhook/1.0`

use std::sync::Arc;
use tracing::{info, warn, error};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::domain::{
    DeliveryStatus, NotificationChannel, NotificationError,
    NotificationRequest, Webhook,
};
use crate::repository::{NotificationRepository, WebhookRepository};

type HmacSha256 = Hmac<Sha256>;

/// Maximum response body size to log (truncated to this many bytes).
const MAX_LOG_BODY: usize = 1024;

/// Deliver a single notification via its configured channel.
///
/// Only `NotificationChannel::Webhook` is supported for HTTP delivery.
/// Email and SMS channels are placeholders for future provider integrations.
pub(crate) async fn deliver_notification(
    notification_repo: &dyn NotificationRepository,
    webhook_repo: &dyn WebhookRepository,
    http_client: &platform_middleware::ssrf::SsrfSafeClient,
    _event_bus: &Arc<dyn platform_messaging::event_bus::EventBus>,
    notification: NotificationRequest,
) -> Result<(), NotificationError> {
    let notification_id = notification.notification_id;
    let notification_status = &notification.status;

    // Only process Queued or Failed notifications
    if !matches!(notification_status, DeliveryStatus::Queued | DeliveryStatus::Failed) {
        return Ok(());
    }

    match notification.channel {
        NotificationChannel::Webhook => {
            deliver_webhook(notification_repo, webhook_repo, http_client, notification).await
        }
        NotificationChannel::Email | NotificationChannel::Sms => {
            // Email/SMS delivery requires a provider integration.
            // Supported providers: SendGrid, AWS SES (email), Twilio (SMS).
            // Set delivery to Failed so the notification is retried or escalated.
            warn!(
                notification_id = %notification_id,
                channel = %notification.channel,
                "Channel delivery not configured — notification marked as failed"
            );
            mark_failed(notification_repo, notification_id, format!("Channel {} delivery not configured", notification.channel)).await
        }
    }
}

/// Deliver a notification via HTTP POST to the registered webhook URL.
async fn deliver_webhook(
    notification_repo: &dyn NotificationRepository,
    webhook_repo: &dyn WebhookRepository,
    http_client: &platform_middleware::ssrf::SsrfSafeClient,
    notification: NotificationRequest,
) -> Result<(), NotificationError> {
    let notification_id = notification.notification_id;
    let payload = notification.payload_json.clone();

    // For webhook delivery, the recipient field stores the webhook_id
    let webhook_id = match uuid::Uuid::parse_str(&notification.recipient) {
        Ok(id) => id,
        Err(_) => {
            // If the recipient is not a valid UUID, treat it as a URL directly
            return send_http_direct(notification_repo, http_client, notification).await;
        }
    };

    // Load the webhook registration
    let webhook = match webhook_repo.load(webhook_id).await {
        Ok(Some(w)) => w,
        Ok(None) => {
            warn!(
                notification_id = %notification_id,
                webhook_id = %webhook_id,
                "Webhook not found, marking notification as failed"
            );
            return mark_failed(notification_repo, notification_id, "Webhook not found".into()).await;
        }
        Err(e) => {
            error!(
                notification_id = %notification_id,
                webhook_id = %webhook_id,
                error = %e,
                "Failed to load webhook registration"
            );
            return mark_failed(notification_repo, notification_id, format!("Repo error: {}", e)).await;
        }
    };

    if !webhook.is_active {
        warn!(
            webhook_id = %webhook_id,
            "Webhook is inactive, skipping delivery"
        );
        return Ok(());
    }

    // Build the HMAC-SHA256 signature
    let signature = match compute_hmac_signature(&webhook, &payload) {
        Some(sig) => sig,
        None => {
            error!(
                webhook_id = %webhook_id,
                "Failed to compute HMAC signature"
            );
            return mark_failed(notification_repo, notification_id, "HMAC computation failed".into()).await;
        }
    };

    let timestamp = Utc::now().timestamp_millis();

    // Send the HTTP POST request
    let result = send_signed_post(http_client, &webhook.url, &payload, &signature, &webhook_id.to_string(), timestamp).await;

    match result {
        Ok(status_code) if (200..300).contains(&status_code) => {
            info!(
                webhook_id = %webhook_id,
                status_code = status_code,
                "Webhook delivered successfully"
            );
            mark_delivered(notification_repo, notification_id).await
        }
        Ok(status_code) => {
            warn!(
                webhook_id = %webhook_id,
                status_code = status_code,
                notification_id = %notification_id,
                "Webhook returned non-2xx status"
            );
            mark_failed(notification_repo, notification_id, format!("HTTP {}", status_code)).await
        }
        Err(e) => {
            error!(
                webhook_id = %webhook_id,
                notification_id = %notification_id,
                error = %e,
                "Webhook HTTP request failed"
            );
            mark_failed(notification_repo, notification_id, format!("Request failed: {}", e)).await
        }
    }
}

/// Send a notification to an arbitrary URL (non-webhook HTTP delivery).
async fn send_http_direct(
    notification_repo: &dyn NotificationRepository,
    http_client: &platform_middleware::ssrf::SsrfSafeClient,
    notification: NotificationRequest,
) -> Result<(), NotificationError> {
    let url = &notification.recipient;
    let payload = &notification.payload_json;

    let result = http_deliver(http_client, url, payload, None).await;

    match result {
        Ok(status_code) if (200..300).contains(&status_code) => {
            info!(
                url = %url,
                status_code = status_code,
                "Direct HTTP delivery succeeded"
            );
            mark_delivered(notification_repo, notification.notification_id).await
        }
        Ok(status_code) => {
            warn!(
                url = %url,
                status_code = status_code,
                "Direct HTTP delivery returned non-2xx"
            );
            mark_failed(notification_repo, notification.notification_id, format!("HTTP {}", status_code)).await
        }
        Err(e) => {
            error!(
                url = %url,
                error = %e,
                "Direct HTTP delivery failed"
            );
            mark_failed(notification_repo, notification.notification_id, format!("Request failed: {}", e)).await
        }
    }
}

/// Send an HTTP POST signed with HMAC-SHA256.
async fn send_signed_post(
    client: &platform_middleware::ssrf::SsrfSafeClient,
    url: &str,
    payload: &str,
    signature: &str,
    webhook_id: &str,
    timestamp: i64,
) -> Result<u16, String> {
    let request_builder = client.post_with(url)
        .map_err(|e| format!("URL validation failed: {}", e))?;

    let response = request_builder
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", format!("sha256={}", signature))
        .header("X-Webhook-ID", webhook_id)
        .header("X-Webhook-Timestamp", timestamp.to_string())
        .header("User-Agent", "PaymentOrchestra-Webhook/1.0")
        .body(payload.to_string())
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status().as_u16();
    let body_preview = response.text().await
        .unwrap_or_default()
        .chars()
        .take(MAX_LOG_BODY)
        .collect::<String>();

    tracing::debug!(
        url = %url,
        status = status,
        response_body = %body_preview,
        "Webhook delivery response"
    );

    Ok(status)
}

/// Send an HTTP POST without signing (direct delivery).
async fn http_deliver(
    client: &platform_middleware::ssrf::SsrfSafeClient,
    url: &str,
    payload: &str,
    custom_headers: Option<Vec<(&str, &str)>>,
) -> Result<u16, String> {
    let mut builder = client.post_with(url)
        .map_err(|e| format!("URL validation failed: {}", e))?;

    builder = builder
        .header("Content-Type", "application/json")
        .header("User-Agent", "PaymentOrchestra-Webhook/1.0")
        .body(payload.to_string());

    if let Some(headers) = custom_headers {
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
    }

    let response = builder
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    Ok(response.status().as_u16())
}

/// Compute HMAC-SHA256 signature for a webhook payload.
///
/// Uses a platform-level signing secret derived from:
///   `WEBHOOK_SIGNING_SECRET` env var + webhook_id
///
/// This avoids storing the raw webhook secret (which would need encryption)
/// while still providing a deterministic, webhook-specific signing key.
///
/// Webhook recipients can verify the signature using the same secret,
/// which is returned to them once on webhook creation.
fn compute_hmac_signature(webhook: &Webhook, payload: &str) -> Option<String> {
    // Derive the signing key from the webhook ID and a platform-level secret.
    // The platform secret is set via the WEBHOOK_SIGNING_SECRET env var.
    // If not set, we fall back to the webhook's secret_hash (legacy behavior).
    let platform_secret = std::env::var("WEBHOOK_SIGNING_SECRET")
        .unwrap_or_else(|_| webhook.secret_hash.clone());

    // Create a webhook-specific key: HMAC(platform_secret, webhook_id)
    let mut derivation = HmacSha256::new_from_slice(platform_secret.as_bytes()).ok()?;
    derivation.update(webhook.webhook_id.to_string().as_bytes());
    let webhook_key = derivation.finalize().into_bytes();

    // Now sign the payload with the derived webhook-specific key
    let mut mac = HmacSha256::new_from_slice(&webhook_key).ok()?;
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    Some(hex::encode(result.into_bytes()))
}

/// Mark a notification as successfully delivered.
async fn mark_delivered(
    repo: &dyn NotificationRepository,
    notification_id: uuid::Uuid,
) -> Result<(), NotificationError> {
    let mut notification = repo.load(notification_id).await?
        .ok_or(NotificationError::NotFound(notification_id))?;

    notification.mark_sent()?;
    repo.save(&notification).await?;

    info!(notification_id = %notification_id, "Notification marked as delivered");
    Ok(())
}

/// Mark a notification as failed (with retry logic).
async fn mark_failed(
    repo: &dyn NotificationRepository,
    notification_id: uuid::Uuid,
    error_msg: String,
) -> Result<(), NotificationError> {
    let mut notification = repo.load(notification_id).await?
        .ok_or(NotificationError::NotFound(notification_id))?;

    let result = notification.mark_failed(error_msg.clone());

    if let Err(e) = &result {
        if matches!(e, NotificationError::AlreadySent) {
            warn!(notification_id = %notification_id, "Notification already sent, ignoring failure");
            return Ok(());
        }
    }

    if let Ok(()) = &result {
        repo.save(&notification).await?;

        if notification.status == DeliveryStatus::DeadLetter {
            error!(
                notification_id = %notification_id,
                error = %error_msg,
                retry_count = notification.retry_count,
                max_retries = notification.max_retries,
                "Notification moved to dead letter queue"
            );
        } else {
            warn!(
                notification_id = %notification_id,
                error = %error_msg,
                retry_count = notification.retry_count,
                "Notification delivery failed, will retry"
            );
        }
    }

    Ok(())
}
