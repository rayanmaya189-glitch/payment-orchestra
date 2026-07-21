use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::application::commands::*;
use crate::domain::aggregates::{AiQuery, Citation};
use crate::domain::rules::*;
use crate::domain::value_objects::CitationSource;
use platform_error::PlatformError;

pub struct AiAssistantServiceImpl {
    query_repo: Box<dyn QueryRepository>,
    db: DatabaseConnection,
    ollama_url: String,
}

impl AiAssistantServiceImpl {
    pub fn new(query_repo: Box<dyn QueryRepository>, db: DatabaseConnection, ollama_url: String) -> Self {
        Self { query_repo, db, ollama_url }
    }
}

#[async_trait]
pub trait AiAssistantService: Send + Sync {
    async fn query(&self, cmd: AiQueryCommand) -> Result<AiQueryResponse, PlatformError>;
}

#[async_trait]
impl AiAssistantService for AiAssistantServiceImpl {
    async fn query(&self, cmd: AiQueryCommand) -> Result<AiQueryResponse, PlatformError> {
        let start = std::time::Instant::now();
        let mut ai_query = AiQuery::new(cmd.principal_id, cmd.query_text.clone(), cmd.session_id);

        // Step 1: Search knowledge base for relevant context
        let search_provider = crate::infrastructure::adapters::ollama_provider::RagSearchProvider::new();
        let search_results = search_provider.search(&cmd.query_text, 5);

        // Step 2: Build context from search results
        let context = if search_results.is_empty() {
            String::new()
        } else {
            let snippets: Vec<String> = search_results.iter().map(|r| r.content.clone()).collect();
            format!("Context:\n{}", snippets.join("\n\n"))
        };

        // Step 3: Call LLM with context
        let llm_provider = crate::infrastructure::adapters::ollama_provider::OllamaProvider::new(self.ollama_url.clone());

        let system_prompt = "You are an AI assistant for a payment orchestration platform. \
            Answer questions about payment processing, connectors, routing, compliance, and platform features. \
            Be concise and accurate. If you don't know, say so.";

        let user_prompt = if context.is_empty() {
            cmd.query_text.clone()
        } else {
            format!("{context}\n\nUser question: {}", cmd.query_text)
        };

        let messages = vec![
            ("system", system_prompt),
            ("user", &user_prompt),
        ];

        match llm_provider.complete(messages, "qwen3").await {
            Ok(answer) => {
                let citations: Vec<Citation> = search_results.iter().map(|r| Citation {
                    source_type: CitationSource::from_str(&r.source_type),
                    source_id: r.source_id.clone(),
                    text_snippet: r.content.clone(),
                    relevance_score: r.score,
                }).collect();

                let confidence = if citations.is_empty() { 0.5 } else { 0.85 };
                let latency = start.elapsed().as_millis() as u64;

                ai_query.complete(answer.clone(), citations.clone(), confidence, 0, latency);
                self.query_repo.save_query(&ai_query).await?;

                Ok(AiQueryResponse {
                    query_id: ai_query.query_id,
                    answer,
                    citations: citations.iter().map(|c| CitationResponse {
                        source_type: c.source_type.as_str().to_string(),
                        source_id: c.source_id.clone(),
                        text_snippet: c.text_snippet.clone(),
                        relevance_score: c.relevance_score,
                    }).collect(),
                    confidence,
                })
            }
            Err(e) => {
                ai_query.fail(e.to_string());
                self.query_repo.save_query(&ai_query).await?;
                Err(e)
            }
        }
    }
}
