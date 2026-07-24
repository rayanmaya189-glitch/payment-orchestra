//! Event type string constants for invoice-service.
//! These are kept for documentation/reference purposes and will be
//! used when event publishing is fully wired.

#[allow(dead_code)]
pub const INVOICE_CREATED: &str = "invoice.created";
#[allow(dead_code)]
pub const INVOICE_SENT: &str = "invoice.sent";
#[allow(dead_code)]
pub const INVOICE_CANCELLED: &str = "invoice.cancelled";
#[allow(dead_code)]
pub const INVOICE_PAID: &str = "invoice.paid";
#[allow(dead_code)]
pub const INVOICE_PARTIALLY_PAID: &str = "invoice.partially_paid";
#[allow(dead_code)]
pub const INVOICE_OVERDUE: &str = "invoice.overdue";
#[allow(dead_code)]
pub const PAYMENT_LINKED: &str = "invoice.payment_linked";
