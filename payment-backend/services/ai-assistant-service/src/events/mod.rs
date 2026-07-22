//! AI Assistant Service events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiAssistantEvent {
    QuestionAsked(QuestionRecord),
    AnswerGenerated(AnswerRecord),
    InsufficientGrounding(InsufficientGroundingRecord),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionRecord {
    pub question_id: Uuid,
    pub session_id: Uuid,
    pub operator_id: Uuid,
    pub question: String,
    pub classification: String,
    pub asked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerRecord {
    pub question_id: Uuid,
    pub session_id: Uuid,
    pub answer: String,
    pub citation_count: u32,
    pub confidence: String,
    pub processing_time_ms: u64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsufficientGroundingRecord {
    pub question_id: Uuid,
    pub session_id: Uuid,
    pub question: String,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}
