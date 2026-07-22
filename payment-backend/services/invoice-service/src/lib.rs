//! invoice-service — Invoice Lifecycle Management.
//!
//! This service handles:
//! - **Invoice** aggregate (CRUD + events): Create, Send, Cancel, payment tracking
//! - Event consumer for PaymentIntent lifecycle events from orchestration-service
//! - Overdue detection and tracking
//!
//! ## Architecture Context
//! This module runs within the modular monolith alongside all other modules.

pub mod domain;
pub mod commands;
pub mod queries;
pub mod events;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use domain::{
    Invoice, InvoiceLineItem, InvoiceStatus, Money, InvoiceError,
};
pub use commands::{
    CommandHandler, InvoiceCommandHandler,
    CreateInvoice, SendInvoice, CancelInvoice, LinkPaymentToInvoice, MarkInvoiceOverdue,
    InvoiceResult,
};
pub use events::{
    InvoiceEvent,
    InvoiceCreated, InvoiceSent, InvoiceCancelled, InvoicePaid,
    InvoicePartiallyPaid, InvoiceOverdue, PaymentLinked,
};
pub use repository::{
    InvoiceRepository, InMemoryInvoiceRepository,
};
pub use queries::{
    QueryHandler, InvoiceQueryHandler,
    GetInvoiceQuery, FindInvoiceByOrderQuery, FindOverdueInvoicesQuery,
};
pub use api::InvoiceApi;
pub use pipeline::InvoicePipeline;
