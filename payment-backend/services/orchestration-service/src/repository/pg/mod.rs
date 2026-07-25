//! PostgreSQL-backed orchestration repositories using SeaORM.
//!
//! Implements all 5 repository traits: PaymentIntentRepository, RoutingPolicyRepository,
//! PaymentMethodTokenRepository, IdempotencyCache, AcquirerLinkProvider.
//!
//! PaymentIntentRepository uses direct CRUD with JSONB columns for complex types
//! (routing_attempts). Other traits use SeaORM for PostgreSQL persistence.
//! Event-sourced variant is in the parent `event_sourced.rs` module.

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

pub mod payment_intent;
pub mod routing_policy;
pub mod payment_method_token;
pub mod idempotency;
pub mod acquirer_link;

/// Combined PostgreSQL-backed repository implementing all orchestration traits.
#[derive(Clone)]
pub struct PostgresOrchestrationRepository {
    pub db: DatabaseConnection,
    pub redis_conn: Option<Arc<Mutex<redis::aio::ConnectionManager>>>,
}

impl PostgresOrchestrationRepository {
    pub fn new(db: DatabaseConnection, redis_conn: Option<redis::aio::ConnectionManager>) -> Self {
        Self {
            db,
            redis_conn: redis_conn.map(|c| Arc::new(Mutex::new(c))),
        }
    }
}
