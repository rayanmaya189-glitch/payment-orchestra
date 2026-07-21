#![allow(dead_code)]
//! Scheduler service — runs periodic jobs for the payment platform.
//!
//! Jobs (SRS Part 4 §6):
//! - JOB-001: Authorization expiry sweep (void stale authorizations)
//! - JOB-003: Outbox relay (publish unpublished events)
//! - JOB-004: Subscription dunning retry
//! - JOB-005: Data retention cleanup
//! - JOB-006: Stuck capturing/refunding sweep
//! - JOB-007: KYB case aging alerts

use platform_config::AppConfig;
use platform_logging::ServiceLogger;

#[tokio::main]
async fn main() {
    ServiceLogger::init("scheduler");

    let config = AppConfig::from_env_or_panic("scheduler");

    let db = platform_db::connect_database(&config.database).await;

    let nats_url = &config.nats.url;
    let stream_name = &config.nats.stream_name;

    let publisher = platform_messaging::EventPublisher::new(nats_url, stream_name)
        .await
        .expect("Failed to connect to NATS JetStream");

    tracing::info!("Scheduler starting — running periodic jobs");

    let db_clone1 = db.clone();
    let db_clone2 = db.clone();
    let db_clone3 = db.clone();
    let db_clone4 = db.clone();
    let db_clone5 = db.clone();
    let publisher_clone = publisher.clone();

    let outbox_handle = tokio::spawn(async move {
        run_outbox_relay(&db_clone1, &publisher_clone).await;
    });

    let auth_expiry_handle = tokio::spawn(async move {
        run_authorization_expiry_sweep(&db_clone2).await;
    });

    let dunning_handle = tokio::spawn(async move {
        run_subscription_dunning_retry(&db_clone3).await;
    });

    let stuck_handle = tokio::spawn(async move {
        run_stuck_status_sweep(&db_clone4).await;
    });

    let data_retention_handle = tokio::spawn(async move {
        run_data_retention_cleanup(&db_clone5).await;
    });

    let db_clone6 = db.clone();
    let kyb_aging_handle = tokio::spawn(async move {
        run_kyb_aging_alerts(&db_clone6).await;
    });

    tokio::select! {
        _ = outbox_handle => {},
        _ = auth_expiry_handle => {},
        _ = dunning_handle => {},
        _ = stuck_handle => {},
        _ = data_retention_handle => {},
        _ = kyb_aging_handle => {},
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Scheduler shutting down");
        }
    }
}

/// JOB-003: Outbox relay — polls unpublished events and publishes to NATS.
async fn run_outbox_relay(
    db: &sea_orm::DatabaseConnection,
    publisher: &platform_messaging::EventPublisher,
) {
    let relay = platform_messaging::OutboxRelay::new(publisher.clone());
    if let Err(e) = relay.run(db).await {
        tracing::error!("Outbox relay failed: {e}");
    }
}

/// JOB-001: Authorization expiry sweep — void stale authorized payment intents.
async fn run_authorization_expiry_sweep(db: &sea_orm::DatabaseConnection) {
    use sea_orm::{ConnectionTrait, Statement};

    let sweep_interval = std::time::Duration::from_secs(3600);
    let expiry_hours: i64 = 168; // 7 days

    loop {
        tracing::info!("Running authorization expiry sweep");

        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE payment_intent SET status = 'voided', updated_at = NOW()
             WHERE status = 'authorized'
             AND updated_at < NOW() - INTERVAL '1 hour' * $1
             RETURNING payment_intent_id",
            vec![expiry_hours.into()],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Voided stale authorized payment intents");
                }
            }
            Err(e) => tracing::error!("Auth expiry sweep failed: {e}"),
        }

        tokio::time::sleep(sweep_interval).await;
    }
}

/// JOB-004: Subscription dunning retry — attempt to charge past-due subscriptions.
async fn run_subscription_dunning_retry(db: &sea_orm::DatabaseConnection) {
    use sea_orm::{ConnectionTrait, Statement};

    let retry_interval = std::time::Duration::from_secs(21600); // Every 6 hours

    loop {
        tracing::info!("Running subscription dunning retry");

        // Find subscriptions that are past_due and haven't exceeded max retries
        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE subscription SET status = 'past_due', retry_count = retry_count + 1, updated_at = NOW()
             WHERE status = 'active'
             AND current_period_end < NOW()
             AND retry_count < max_retries
             RETURNING subscription_id",
            vec![],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Marked subscriptions as past-due for retry");
                }
            }
            Err(e) => tracing::error!("Dunning retry failed: {e}"),
        }

        // Cancel subscriptions that have exceeded max retries
        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE subscription SET status = 'canceled', canceled_at = NOW(), updated_at = NOW()
             WHERE status = 'past_due'
             AND retry_count >= max_retries
             RETURNING subscription_id",
            vec![],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Canceled subscriptions exceeding max retries");
                }
            }
            Err(e) => tracing::error!("Dunning cancel failed: {e}"),
        }

        tokio::time::sleep(retry_interval).await;
    }
}

/// JOB-006: Stuck status sweep — timeout payment intents stuck in transitional states.
async fn run_stuck_status_sweep(db: &sea_orm::DatabaseConnection) {
    use sea_orm::{ConnectionTrait, Statement};

    let sweep_interval = std::time::Duration::from_secs(1800); // Every 30 minutes
    let stuck_hours: i64 = 2; // 2 hours

    loop {
        tracing::info!("Running stuck status sweep");

        // Timeout authorizing intents (stuck during failover)
        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE payment_intent SET status = 'failed', updated_at = NOW()
             WHERE status = 'authorizing'
             AND updated_at < NOW() - INTERVAL '1 hour' * $1
             RETURNING payment_intent_id",
            vec![stuck_hours.into()],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Timed out stuck authorizing payment intents");
                }
            }
            Err(e) => tracing::error!("Stuck status sweep failed: {e}"),
        }

        // Timeout capturing intents
        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE payment_intent SET status = 'failed', updated_at = NOW()
             WHERE status = 'capturing'
             AND updated_at < NOW() - INTERVAL '1 hour' * $1
             RETURNING payment_intent_id",
            vec![stuck_hours.into()],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Timed out stuck capturing payment intents");
                }
            }
            Err(e) => tracing::error!("Stuck capturing sweep failed: {e}"),
        }

        tokio::time::sleep(sweep_interval).await;
    }
}

/// JOB-005: Data retention cleanup — delete old data per retention policy.
async fn run_data_retention_cleanup(db: &sea_orm::DatabaseConnection) {
    use sea_orm::{ConnectionTrait, Statement};

    let cleanup_interval = std::time::Duration::from_secs(86400); // Daily
    let retention_days: i64 = 2555; // 7 years for financial data

    loop {
        tracing::info!("Running data retention cleanup");

        // Clean old outbox entries (keep 30 days)
        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM outbox WHERE published_at IS NOT NULL
             AND published_at < NOW() - INTERVAL '1 day' * 30",
            vec![],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Cleaned old outbox entries");
                }
            }
            Err(e) => tracing::error!("Outbox cleanup failed: {e}"),
        }

        // Clean old AI request logs (keep 90 days)
        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM ai_request WHERE created_at < NOW() - INTERVAL '1 day' * 90",
            vec![],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Cleaned old AI request logs");
                }
            }
            Err(e) => tracing::error!("AI request cleanup failed: {e}"),
        }

        // Clean old refresh tokens (keep 90 days)
        match db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM refresh_token WHERE created_at < NOW() - INTERVAL '1 day' * 90",
            vec![],
        )).await {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    tracing::info!(count, "Cleaned old refresh tokens");
                }
            }
            Err(e) => tracing::error!("Refresh token cleanup failed: {e}"),
        }

        // Note: Financial data (audit_log, event_store, payment_intent, settlement_batch)
        // is retained for 7 years per regulatory requirements — NOT deleted.
        tracing::debug!(
            retention_years = retention_days / 365,
            "Financial data retention policy: {} years (no cleanup)", retention_days / 365
        );

        tokio::time::sleep(cleanup_interval).await;
    }
}

/// JOB-007: KYB case aging alerts — notify when cases are pending review for too long.
async fn run_kyb_aging_alerts(db: &sea_orm::DatabaseConnection) {
    use sea_orm::{ConnectionTrait, Statement};

    let check_interval = std::time::Duration::from_secs(43200); // Every 12 hours
    let aging_days: i64 = 7; // Alert after 7 days pending

    loop {
        tracing::info!("Running KYB aging alerts check");

        // Find KYB cases pending for too long
        match db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT kyb_case_id, operator_id, status, created_at
             FROM kyb_case
             WHERE status = 'submitted'
             AND created_at < NOW() - INTERVAL '1 day' * $1",
            vec![aging_days.into()],
        )).await {
            Ok(rows) => {
                let count = rows.len();
                if count > 0 {
                    tracing::warn!(
                        count,
                        "KYB cases pending review for over {} days — requires attention",
                        aging_days
                    );
                    // In production: send notification to compliance officers
                    // via notification-service or external alerting system
                }
            }
            Err(e) => tracing::error!("KYB aging check failed: {e}"),
        }

        tokio::time::sleep(check_interval).await;
    }
}
