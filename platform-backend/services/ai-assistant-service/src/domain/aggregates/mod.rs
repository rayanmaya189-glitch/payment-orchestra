use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{QueryStatus, CitationSource};

/// AI Query aggregate — represents a user query and its RAG response.
#[derive(Debug, Clone)]
pub struct AiQuery {
    pub query_id: Uuid,
    pub session_id: Option<String>,
    pub principal_id: Uuid,
    pub query_text: String,
    pub status: QueryStatus,
    pub answer: Option<String>,
    pub citations: Vec<Citation>,
    pub confidence: Option<f64>,
    pub tokens_used: Option<u32>,
    pub latency_ms: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Citation {
    pub source_type: CitationSource,
    pub source_id: String,
    pub text_snippet: String,
    pub relevance_score: f64,
}

impl AiQuery {
    pub fn new(principal_id: Uuid, query_text: String, session_id: Option<String>) -> Self {
        Self {
            query_id: Uuid::now_v7(),
            session_id,
            principal_id,
            query_text,
            status: QueryStatus::Processing,
            answer: None,
            citations: Vec::new(),
            confidence: None,
            tokens_used: None,
            latency_ms: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn complete(&mut self, answer: String, citations: Vec<Citation>, confidence: f64, tokens_used: u32, latency_ms: u64) {
        self.status = QueryStatus::Completed;
        self.answer = Some(answer);
        self.citations = citations;
        self.confidence = Some(confidence);
        self.tokens_used = Some(tokens_used);
        self.latency_ms = Some(latency_ms);
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, reason: String) {
        self.status = QueryStatus::Failed;
        self.answer = Some(format!("Error: {reason}"));
        self.completed_at = Some(Utc::now());
    }
}

/// AI Session — tracks conversation history.
#[derive(Debug, Clone)]
pub struct AiSession {
    pub session_id: String,
    pub principal_id: Uuid,
    pub messages: Vec<SessionMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl AiSession {
    pub fn new(principal_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::now_v7().to_string(),
            principal_id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_query() {
        let q = AiQuery::new(Uuid::now_v7(), "What is 3DS?".to_string(), None);
        assert_eq!(q.status, QueryStatus::Processing);
        assert!(q.answer.is_none());
    }

    #[test]
    fn test_complete_query() {
        let mut q = AiQuery::new(Uuid::now_v7(), "test".to_string(), None);
        q.complete("answer".to_string(), vec![], 0.95, 100, 50);
        assert_eq!(q.status, QueryStatus::Completed);
        assert_eq!(q.confidence, Some(0.95));
    }

    #[test]
    fn test_session_messages() {
        let mut s = AiSession::new(Uuid::now_v7());
        s.add_message("user", "hello");
        s.add_message("assistant", "hi there");
        assert_eq!(s.messages.len(), 2);
    }
}
