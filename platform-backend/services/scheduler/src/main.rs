#![allow(dead_code)]
//! Scheduler service — runs periodic jobs for the payment platform.
//!
//! Jobs (SRS Part 4 §6):
//! - JOB-001: Authorization expiry sweep (void stale authorizations)
//! - JOB-002: Settlement polling (fetch from connectors)
//! - JOB-003: Outbox relay (publish unpublished events)
//! - JOB-004: Subscription renewal (upcoming renewals)
//! - JOB-005: Data retention cleanup

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

    // Run all jobs in parallel
    let db_clone = db.clone();
    let publisher_clone = publisher.clone();

    let outbox_handle = tokio::spawn(async move {
        run_outbox_relay(&db_clone, &publisher_clone).await;
    });

    let auth_expiry_handle = tokio::spawn(async move {
        run_authorization_expiry_sweep(&db).await;
    });

    // Wait for all jobs (they run forever)
    tokio::select! {
        _ = outbox_handle => {},
        _ = auth_expiry_handle => {},
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
///
/// Payment intents that have been authorized but not captured within the
/// expiry window (default: 7 days) are automatically voided.
async fn run_authorization_expiry_sweep(db: &sea_orm::DatabaseConnection) {
    use sea_orm::{ConnectionTrait, Statement};

    let sweep_interval = std::time::Duration::from_secs(3600); // Every hour
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
                let rows_affected = result.rows_affected();
                if rows_affected > 0 {
                    tracing::info!(
                        count = rows_affected,
                        "Voided stale authorized payment intents"
                    );
                }
            }
            Err(e) => {
                tracing::error!("Authorization expiry sweep failed: {e}");
            }
        }

        tokio::time::sleep(sweep_interval).await;
    }
}
