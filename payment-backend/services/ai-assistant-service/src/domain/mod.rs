//! AI Assistant Service domain model — BC-12
//!
//! Read-only Conformist. RAG pipeline over operator data.
//! No write access to money-movement contexts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ConversationSession — bounded history window, operator-scoped
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    pub session_id: Uuid,
    pub operator_id: Uuid,
    pub title: String,
    pub messages: Vec<ConversationMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub message_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub citations: Vec<GroundingCitation>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
}

// ---------------------------------------------------------------------------
// GroundingCitation — audit trail per answer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingCitation {
    pub citation_id: Uuid,
    pub source_type: CitationSourceType,
    pub source_id: String,
    pub source_name: String,
    pub excerpt: String,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CitationSourceType {
    PaymentEvent,
    Document,
    PriorQa,
    KnowledgeBase,
}

impl std::fmt::Display for CitationSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PaymentEvent => write!(f, "payment_event"),
            Self::Document => write!(f, "document"),
            Self::PriorQa => write!(f, "prior_qa"),
            Self::KnowledgeBase => write!(f, "knowledge_base"),
        }
    }
}

// ---------------------------------------------------------------------------
// RAG Pipeline types
// ---------------------------------------------------------------------------

/// Classification of a user query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryClassification {
    /// Can be answered via structured lookup (e.g., "What was my auth rate yesterday?")
    StructuredLookup,
    /// Requires retrieval from documents or knowledge base (e.g., "How do I set up a refund?")
    RetrievalEligible,
    /// Cannot be answered with available data
    Unanswerable,
}

/// A chunk of retrieved context for RAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub chunk_id: String,
    pub content: String,
    pub source_type: CitationSourceType,
    pub source_id: String,
    pub source_name: String,
    pub relevance_score: f64,
}

/// The result of a RAG query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResult {
    pub answer: String,
    pub citations: Vec<GroundingCitation>,
    pub confidence: AnswerConfidence,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnswerConfidence {
    High,
    Medium,
    Low,
    InsufficientData,
}

// ---------------------------------------------------------------------------
// Error catalog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum AiError {
    #[error("AI Assistant temporarily unavailable: {0}")]
    Unavailable(String),
    #[error("Insufficient data to answer the question")]
    InsufficientGrounding,
    #[error("AI usage quota exceeded. Try again later")]
    QuotaExceeded,
    #[error("Query too broad: would return more than {max_results} results")]
    QueryTooBroad { max_results: u32 },
    #[error("Conversation session not found: {0}")]
    SessionNotFound(Uuid),
    #[error("Operator ID mismatch: session belongs to a different operator")]
    OperatorMismatch,
    #[error("Rate limit exceeded for query type: {0}")]
    RateLimitExceeded(String),
    #[error("Message too long: {length} characters (max: {max})")]
    MessageTooLong { length: usize, max: usize },
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Max history window (messages per session).
pub const MAX_HISTORY_WINDOW: usize = 50;
/// Max output records for data queries.
pub const MAX_OUTPUT_RECORDS: u32 = 100;
/// Max date range for data queries (days).
pub const MAX_DATE_RANGE_DAYS: u32 = 30;
/// Max question length.
pub const MAX_QUESTION_LENGTH: usize = 2000;
/// Rate limit: queries per minute per operator.
pub const QUERIES_PER_MINUTE: u32 = 20;
