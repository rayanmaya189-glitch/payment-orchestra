//! Invoice-service API — handler exports for the modular monolith.

pub use crate::commands::{CommandHandler, InvoiceCommandHandler};
pub use crate::queries::{QueryHandler, InvoiceQueryHandler};
pub use crate::domain::Invoice;

pub struct InvoiceApi {
    pub commands: Box<dyn CommandHandler>,
    pub queries: Box<dyn QueryHandler>,
}
