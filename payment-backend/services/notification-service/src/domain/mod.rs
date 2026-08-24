//! Notification Service domain model — BC-14
//!
//! At-least-once delivery of email/SMS/webhook notifications.
//! Subscribes to domain events and dispatches via configured providers.
//!
//! Per CONVENTIONS.md: one concept per file.

pub mod channel;
pub mod delivery_status;
pub mod error;
pub mod notification_request;
pub mod template;
pub mod webhook;

pub use channel::*;
pub use delivery_status::*;
pub use error::*;
pub use notification_request::*;
pub use template::*;
pub use webhook::*;
