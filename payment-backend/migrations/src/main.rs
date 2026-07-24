//! Database migration runner for Payment Orchestra.
//!
//! Usage:
//!   cargo run --bin migrations                    # Run all pending migrations
//!   cargo run --bin migrations -- fresh            # Drop all tables then re-run
//!   cargo run --bin migrations -- down             # Rollback last migration
//!
//! Environment variables:
//!   DATABASE_URL     PostgreSQL connection URL (default from ServiceConfig)
//!   DB_HOST, DB_PORT, DB_NAME, DB_USERNAME, DB_PASSWORD  (alternative to DATABASE_URL)

use migrations::Migrator;
use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".into());
        let port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".into());
        let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "payment_orchestra".into());
        let username = std::env::var("DB_USERNAME").unwrap_or_else(|_| "platform".into());
        let password = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "dev_password".into());
        format!("postgres://{}:{}@{}:{}/{}", username, password, host, port, db_name)
    });

    tracing::info!("Connecting to database");
    let db = sea_orm::Database::connect(&database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("up");

    match command {
        "up" | "run" => {
            tracing::info!("Running pending migrations");
            Migrator::up(&db, None).await?;
            tracing::info!("Migrations complete");
        }
        "down" => {
            tracing::info!("Rolling back last migration");
            Migrator::down(&db, None).await?;
            tracing::info!("Rollback complete");
        }
        "fresh" => {
            tracing::info!("Dropping all tables and re-running migrations");
            Migrator::fresh(&db).await?;
            tracing::info!("Fresh migration complete");
        }
        "status" => {
            use sea_orm_migration::MigratorTrait;
            let statuses = Migrator::get_migration_with_status(&db).await?;
            for st in &statuses {
                tracing::info!("Migration: {} — {:?}", st.name(), st.status());
            }
        }
        other => {
            tracing::error!("Unknown command: {}. Use: up, down, fresh, status", other);
            std::process::exit(1);
        }
    }

    Ok(())
}
