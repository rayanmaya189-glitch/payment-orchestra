pub mod local_storage;
pub mod minio_storage;
pub mod noop_document_repository;
pub mod postgres_document_repository;

pub use local_storage::LocalStorageProvider;
pub use minio_storage::{MinioConfig, MinioStorageProvider};
pub use noop_document_repository::NoopDocumentRepository;
pub use postgres_document_repository::PostgresDocumentRepository;
