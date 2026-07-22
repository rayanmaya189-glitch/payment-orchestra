//! Event store abstractions and SeaORM entities for event-sourced services.
//! Append-only event store with optimistic concurrency and snapshotting.

pub mod event_store;
pub mod snapshot;
pub mod replay;
pub mod corruption;
