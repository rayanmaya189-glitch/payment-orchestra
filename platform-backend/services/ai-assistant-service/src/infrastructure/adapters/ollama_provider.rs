use async_trait::async_trait;
use crate::domain::rules::LlmProvider;
use platform_error::PlatformError;

/// Ollama LLM provider — calls local Ollama API for completions.
pub struct OllamaProvider {
    base_url: String,
    http_client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(base_url: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self { base_url, http_client }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, messages: Vec<(&str, &str)>, model: &str) -> Result<String, PlatformError> {
        let messages_json: Vec<serde_json::Value> = messages
            .iter()
            .map(|(role, content)| {
                serde_json::json!({
                    "role": role,
                    "content": content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": messages_json,
            "stream": false,
        });

        let response = self.http_client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("Ollama request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(PlatformError::Internal(format!(
                "Ollama returned status {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::Internal(format!("Ollama response parse failed: {e}")))?;

        let content = result["message"]["content"]
            .as_str()
            .unwrap_or("No response from LLM")
            .to_string();

        Ok(content)
    }
}

/// RAG search provider — searches knowledge base for relevant context.
pub struct RagSearchProvider {
    knowledge_base: Vec<KnowledgeEntry>,
}

struct KnowledgeEntry {
    content: String,
    source_type: String,
    source_id: String,
}

impl RagSearchProvider {
    pub fn new() -> Self {
        Self {
            knowledge_base: Vec::new(),
        }
    }

    /// Simple keyword-based search (production would use vector embeddings).
    pub fn search(&self, query: &str, top_k: usize) -> Vec<crate::domain::rules::SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<crate::domain::rules::SearchResult> = self.knowledge_base
            .iter()
            .filter(|entry| {
                let content_lower = entry.content.to_lowercase();
                query_lower.split_whitespace().any(|word| content_lower.contains(word))
            })
            .enumerate()
            .take(top_k)
            .map(|(i, entry)| crate::domain::rules::SearchResult {
                content: entry.content.clone(),
                source_type: entry.source_type.clone(),
                source_id: entry.source_id.clone(),
                score: 1.0 - (i as f64 * 0.1),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}
