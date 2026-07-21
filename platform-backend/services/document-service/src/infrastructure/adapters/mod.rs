pub mod postgres_document_repository;
pub mod noop_document_repository;
pub mod local_storage;

pub use postgres_document_repository::PostgresDocumentRepository;
pub use noop_document_repository::NoopDocumentRepository;
pub use local_storage::LocalStorageProvider;
