//! Transactional outbox pattern — reliable event publishing via database-first writes.
//!
//! Ensures at-least-once event delivery by writing events to the outbox table
//! within the same transaction as the domain event append, then publishing
//! asynchronously via a background relay process.

pub mod outbox;
pub mod outbox_entity;
pub mod relay;
pub mod dedup;

pub use outbox::OutboxEntry;
pub use outbox_entity::Entity as OutboxEntity;
pub use relay::{OutboxRelay, OutboxRelayConfig};
pub use dedup::DedupStore;
