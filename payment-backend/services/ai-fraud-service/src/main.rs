//! AI Fraud Detection Service
//!
//! Real-time fraud detection using ML models and rule-based systems.
//! Analyzes payment patterns, device fingerprints, and behavioral signals.

use std::net::SocketAddr;
use std::sync::Arc;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod domain;
mod repository;
mod api;

use config::FraudServiceConfig;
use repository::InMemoryFraudRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "ai_fraud_service=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = FraudServiceConfig::from_env()?;
    let addr: SocketAddr = config.listen_addr.parse()?;
    let repository = Arc::new(InMemoryFraudRepository::new());

    tracing::info!("AI Fraud Detection Service starting on {}", addr);

    // Initialize ML models
    let fraud_detector = domain::FraudDetector::new(
        repository.clone(),
        config.model_config.clone(),
    ).await?;

    // Start gRPC server
    let grpc_service = api::grpc::FraudGrpcService::new(fraud_detector);

    // For now, use a simple HTTP server
    // In production, implement proper tonic gRPC service
    tracing::info!("AI Fraud Detection Service listening on {}", addr);
    
    // Keep the service running
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down AI Fraud Detection Service");

    Ok(())
}
