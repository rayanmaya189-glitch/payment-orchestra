//! Database connection management, health checks, and leader election.
//! Shared PostgreSQL and Redis connection pool configuration.

pub mod connection;
pub mod health;
pub mod leader;
pub mod migration;
