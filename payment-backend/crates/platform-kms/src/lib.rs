//! Envelope encryption client (ADR-014).
//! KMS-managed KEK + per-link DEK for credential encryption.

pub mod envelope;
pub mod kek;
pub mod dek;
pub mod aad;
