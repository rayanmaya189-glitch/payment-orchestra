//! Platform Event Store — append-only event persistence with integrity checks.
//!
//! Provides:
//! - `EventStore`: Append/read events for event-sourced aggregates
//! - `SnapshotStore`: Save/load aggregate snapshots
//! - `EventReplayer`: Rebuild aggregate state from event streams
//! - `verify_event_integrity`: SHA-256 checksum verification

pub mod event_store;
pub mod event_store_entity;
pub mod snapshot;
pub mod snapshot_entity;
pub mod replay;
pub mod corruption;

pub use event_store::{EventStore, StoredEvent};
pub use snapshot::{SnapshotStore, AggregateSnapshot};
pub use replay::EventReplayer;
pub use corruption::{compute_checksum, verify_checksum};
