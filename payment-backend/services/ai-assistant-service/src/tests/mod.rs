//! AI Assistant Service TDD tests
//!
//! Tests cover:
//! 1. Start a conversation session
//! 2. Ask a question → get answer with citations (AI-P-001)
//! 3. Insufficient grounding → error (AI-P-001)
//! 4. Cross-operator data isolation (AI-P-002)
//! 5. Output volume limit enforced (AI-EXFIL-001)
//! 6. Message length limit
//! 7. Rate limiting (AI-RATE-001)
//! 8. List sessions
//! 9. Session info
//! 10. Health status

mod conversation_tests;
mod guardrail_tests;
mod rag_tests;

use crate::api::AiAssistantApi;
use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use crate::queries::{AiQueryHandler, QueryHandler};
use crate::repository::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub(crate) fn setup() -> AiAssistantPipeline {
    AiAssistantPipeline::new()
}
