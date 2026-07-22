//! Transactional outbox pattern (ADR-011).
//! Outbox write in the same DB transaction as aggregate change + relay to in-process NATS channels.

pub mod outbox;
pub mod relay;
pub mod dedup;
