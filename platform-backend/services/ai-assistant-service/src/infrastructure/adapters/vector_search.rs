//! In-memory vector search implementation for development and testing.
//!
//! Provides a simple keyword-based vector search that implements the `VectorSearch`
//! domain trait. In production, this would be replaced with an OpenSearch-backed
//! implementation using cosine similarity over embedded vectors.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::rules::{SearchResult, VectorSearch};
use platform_error::PlatformError;

/// A document entry in the vector store.
#[derive(Debug, Clone)]
pub struct VectorDocument {
    pub doc_id: String,
    pub content: String,
    pub source_type: String,
    pub source_id: String,
    pub embedding: Option<Vec<f32>>,
}

/// In-memory vector search implementation.
///
/// Stores documents in a `RwLock<HashMap>` for concurrent access.
/// Uses keyword matching with TF scoring as a stand-in for real
/// vector similarity search (cosine, dot product, etc.).
pub struct InMemoryVectorSearch {
    documents: RwLock<HashMap<String, VectorDocument>>,
}

impl InMemoryVectorSearch {
    pub fn new() -> Self {
        Self {
            documents: RwLock::new(HashMap::new()),
        }
    }

    /// Insert or update a document in the vector store.
    pub fn upsert(&self, doc: VectorDocument) {
        let mut docs = self.documents.write().unwrap();
        docs.insert(doc.doc_id.clone(), doc);
    }

    /// Remove a document from the vector store.
    pub fn remove(&self, doc_id: &str) -> bool {
        let mut docs = self.documents.write().unwrap();
        docs.remove(doc_id).is_some()
    }

    /// Return the number of documents in the store.
    pub fn len(&self) -> usize {
        self.documents.read().unwrap().len()
    }

    /// Compute a keyword-relevance score for a document against a query.
    /// Score is the fraction of query tokens found in the content.
    fn score_document(content: &str, query_tokens: &[&str]) -> f64 {
        if query_tokens.is_empty() {
            return 0.0;
        }
        let content_lower = content.to_lowercase();
        let matched = query_tokens
            .iter()
            .filter(|token| content_lower.contains(&token.to_lowercase()))
            .count();
        matched as f64 / query_tokens.len() as f64
    }
}

#[async_trait]
impl VectorSearch for InMemoryVectorSearch {
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>, PlatformError> {
        let docs = self.documents.read().unwrap();
        let query_tokens: Vec<&str> = query.split_whitespace().collect();

        let mut scored: Vec<(f64, &VectorDocument)> = docs
            .values()
            .map(|doc| {
                let score = Self::score_document(&doc.content, &query_tokens);
                (score, doc)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SearchResult> = scored
            .into_iter()
            .take(top_k)
            .map(|(score, doc)| SearchResult {
                content: doc.content.clone(),
                source_type: doc.source_type.clone(),
                source_id: doc.source_id.clone(),
                score,
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> InMemoryVectorSearch {
        let store = InMemoryVectorSearch::new();
        store.upsert(VectorDocument {
            doc_id: "doc-1".to_string(),
            content: "Payment routing rules determine which connector processes each transaction".to_string(),
            source_type: "documentation".to_string(),
            source_id: "routing-guide".to_string(),
            embedding: None,
        });
        store.upsert(VectorDocument {
            doc_id: "doc-2".to_string(),
            content: "Authorization rates measure the success of payment attempts".to_string(),
            source_type: "documentation".to_string(),
            source_id: "analytics-guide".to_string(),
            embedding: None,
        });
        store.upsert(VectorDocument {
            doc_id: "doc-3".to_string(),
            content: "Connector health checks monitor uptime and latency".to_string(),
            source_type: "api_reference".to_string(),
            source_id: "connectors-api".to_string(),
            embedding: None,
        });
        store
    }

    #[tokio::test]
    async fn test_search_returns_relevant_results() {
        let store = create_test_store();
        let results = store.search("payment routing connector", 5).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].source_id, "routing-guide");
    }

    #[tokio::test]
    async fn test_search_respects_top_k() {
        let store = create_test_store();
        let results = store.search("payment", 1).await.unwrap();
        assert!(results.len() <= 1);
    }

    #[tokio::test]
    async fn test_search_returns_empty_for_no_match() {
        let store = create_test_store();
        let results = store.search("xyzzy nonexistent", 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_upsert_and_remove() {
        let store = InMemoryVectorSearch::new();
        assert_eq!(store.len(), 0);
        store.upsert(VectorDocument {
            doc_id: "d1".to_string(),
            content: "test".to_string(),
            source_type: "test".to_string(),
            source_id: "d1".to_string(),
            embedding: None,
        });
        assert_eq!(store.len(), 1);
        assert!(store.remove("d1"));
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_score_document() {
        let score = InMemoryVectorSearch::score_document(
            "payment routing rules",
            &["payment", "routing"],
        );
        assert!((score - 1.0).abs() < f64::EPSILON);

        let score = InMemoryVectorSearch::score_document("payment", &["routing", "xyzzy"]);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }
}
