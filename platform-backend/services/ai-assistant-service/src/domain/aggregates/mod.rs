use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{QueryType, CitationSource};

/// AI Assistant conversation session.
#[derive(Debug, Clone)]
pub struct ConversationSession {
    pub session_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub message_count: u32,
}

impl ConversationSession {
    pub fn new(operator_id: Uuid, principal_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::now_v7(),
            operator_id,
            principal_id,
            created_at: now,
            last_activity_at: now,
            message_count: 0,
        }
    }

    pub fn record_message(&mut self) {
        self.message_count += 1;
        self.last_activity_at = Utc::now();
    }
}

/// AI Assistant query — a user question to the assistant.
#[derive(Debug, Clone)]
pub struct AssistantQuery {
    pub query_id: Uuid,
    pub session_id: Uuid,
    pub query: String,
    pub query_type: QueryType,
    pub created_at: DateTime<Utc>,
}

impl AssistantQuery {
    pub fn new(session_id: Uuid, query: String) -> Self {
        let query_type = Self::classify_query(&query);
        Self {
            query_id: Uuid::now_v7(),
            session_id,
            query,
            query_type,
            created_at: Utc::now(),
        }
    }

    /// Classify query type based on content (SRS Part 6 §3).
    fn classify_query(query: &str) -> QueryType {
        let lower = query.to_lowercase();
        if lower.contains("transaction") || lower.contains("payment") || lower.contains("authorize") {
            QueryType::TransactionLookup
        } else if lower.contains("reconciliation") || lower.contains("settlement") {
            QueryType::ReconciliationQuery
        } else if lower.contains("invoice") || lower.contains("billing") {
            QueryType::InvoiceQuery
        } else if lower.contains("dispute") || lower.contains("chargeback") {
            QueryType::DisputeQuery
        } else if lower.contains("risk") || lower.contains("fraud") {
            QueryType::RiskQuery
        } else {
            QueryType::GeneralOperation
        }
    }
}

/// AI Assistant response — grounded answer with citations.
#[derive(Debug, Clone)]
pub struct AssistantResponse {
    pub query_id: Uuid,
    pub answer: String,
    pub citations: Vec<CitationSource>,
    pub confidence: f64,
    pub model_used: String,
    pub latency_ms: u64,
    pub created_at: DateTime<Utc>,
}

impl AssistantResponse {
    pub fn new(query_id: Uuid, answer: String, model_used: String) -> Self {
        Self {
            query_id,
            answer,
            citations: Vec::new(),
            confidence: 0.0,
            model_used,
            latency_ms: 0,
            created_at: Utc::now(),
        }
    }

    /// Validate that citations are present and non-empty (SRS BIZ-023).
    pub fn validate_citations(&self) -> bool {
        !self.citations.is_empty() && self.citations.iter().all(|c| !c.source_id.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let s = ConversationSession::new(Uuid::now_v7(), Uuid::now_v7());
        assert_eq!(s.message_count, 0);
    }

    #[test]
    fn test_record_message() {
        let mut s = ConversationSession::new(Uuid::now_v7(), Uuid::now_v7());
        s.record_message();
        assert_eq!(s.message_count, 1);
    }

    #[test]
    fn test_classify_query_transaction() {
        let q = AssistantQuery::new(Uuid::now_v7(), "Show me my recent transactions".into());
        assert_eq!(q.query_type, QueryType::TransactionLookup);
    }

    #[test]
    fn test_classify_query_reconciliation() {
        let q = AssistantQuery::new(Uuid::now_v7(), "What is the settlement status?".into());
        assert_eq!(q.query_type, QueryType::ReconciliationQuery);
    }

    #[test]
    fn test_classify_query_general() {
        let q = AssistantQuery::new(Uuid::now_v7(), "Hello".into());
        assert_eq!(q.query_type, QueryType::GeneralOperation);
    }

    #[test]
    fn test_validate_citations() {
        let mut resp = AssistantResponse::new(Uuid::now_v7(), "Answer".into(), "qwen3".into());
        assert!(!resp.validate_citations());

        resp.citations.push(CitationSource {
            source_type: "transaction".into(),
            source_id: "tx_123".into(),
            text_snippet: "Transaction #123".into(),
            relevance_score: 0.9,
        });
        assert!(resp.validate_citations());
    }
}
