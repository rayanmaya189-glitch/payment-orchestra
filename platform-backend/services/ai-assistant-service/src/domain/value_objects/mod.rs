#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

/// Query classification types per SRS Part 6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    TransactionLookup,
    ReconciliationQuery,
    InvoiceQuery,
    DisputeQuery,
    RiskQuery,
    GeneralOperation,
}

impl QueryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TransactionLookup => "transaction_lookup",
            Self::ReconciliationQuery => "reconciliation_query",
            Self::InvoiceQuery => "invoice_query",
            Self::DisputeQuery => "dispute_query",
            Self::RiskQuery => "risk_query",
            Self::GeneralOperation => "general_operation",
        }
    }
}

/// Citation source for grounded answers (SRS BIZ-023).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationSource {
    pub source_type: String,
    pub source_id: String,
    pub text_snippet: String,
    pub relevance_score: f64,
}

/// RAG retrieval result before reranking.
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub chunk_id: String,
    pub source_type: String,
    pub source_id: String,
    pub text_content: String,
    pub dense_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_values() {
        assert_eq!(QueryType::TransactionLookup.as_str(), "transaction_lookup");
        assert_eq!(QueryType::GeneralOperation.as_str(), "general_operation");
    }
}
