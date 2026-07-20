use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::{AiQuery, AiSession};
use platform_error::PlatformError;

#[async_trait]
pub trait QueryRepository: Send + Sync {
    async fn save_query(&self, query: &AiQuery) -> Result<(), PlatformError>;
    async fn find_query(&self, id: Uuid) -> Result<Option<AiQuery>, PlatformError>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save_session(&self, session: &AiSession) -> Result<(), PlatformError>;
    async fn find_session(&self, id: &str) -> Result<Option<AiSession>, PlatformError>;
}

/// LLM provider trait — abstraction over Ollama/OpenAI.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: Vec<(&str, &str)>, model: &str) -> Result<String, PlatformError>;
}

/// Vector search provider trait.
#[async_trait]
pub trait VectorSearch: Send + Sync {
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>, PlatformError>;
}

pub struct SearchResult {
    pub content: String,
    pub source_type: String,
    pub source_id: String,
    pub score: f64,
}
