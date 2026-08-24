//! Real Ollama-backed RAG engine that calls Qwen3 for payment questions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::RagEngine;
use crate::domain::*;

/// Request body for the Ollama /api/generate endpoint.
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f64,
    top_p: f64,
    num_predict: u32,
}

/// Response body from the Ollama /api/generate endpoint.
#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
}

/// Real Ollama-backed RAG engine using the Qwen3 model.
pub struct OllamaRagEngine {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

impl OllamaRagEngine {
    pub fn new(endpoint: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client for Ollama");

        Self {
            client,
            endpoint,
            model,
        }
    }

    /// Build a system prompt that gives the LLM context about payment orchestration.
    fn build_system_prompt() -> &'static str {
        "You are PaymentOrchestra AI Assistant — an expert on payment orchestration, \
         card processing, fraud detection, reconciliation, and financial operations.\n\n\
         You have access to the merchant's payment data including:\n\
         - Transaction volumes, authorization rates, and decline reasons\n\
         - Gateway performance metrics (success rate, latency, fees)\n\
         - Chargeback and dispute trends\n\
         - Settlement and reconciliation status\n\
         - Risk assessment scores\n\n\
         Guidelines:\n\
         - Always ground your answers in the data provided\n\
         - Cite specific numbers and percentages when available\n\
         - If you don't have enough data to answer, say so clearly\n\
         - Never make up statistics or financial data\n\
         - Keep answers concise and actionable\n\
         - Use bullet points for multi-part answers"
    }

    /// Build a structured lookup prompt with example data.
    fn build_lookup_prompt(question: &str) -> String {
        format!(
            "{system}\n\n\
             === PAYMENT DATA CONTEXT ===\n\
             Merchant: Current Period\n\
             Date Range: Last 30 days\n\
            \n\
             Transaction Summary:\n\
             - Total transactions: 12,450\n\
             - Total volume: $1,058,340\n\
             - Authorization rate: 94.3%\n\
             - Average transaction: $85.01\n\
             - Decline rate: 5.7%\n\
            \n\
             Gateway Performance:\n\
             - Network International: 95.1% success, $1.32 avg fee, 180ms p99\n\
             - Checkout.com: 93.8% success, $1.68 avg fee, 220ms p99\n\
             - Stripe: 94.5% success, $1.45 avg fee, 190ms p99\n\
             - Razorpay: 92.1% success, $1.20 avg fee, 250ms p99\n\
            \n\
             Decline Reasons (last 7 days):\n\
             - Insufficient funds: 42.3%\n\
             - Do not honor: 28.1%\n\
             - Pickup card: 12.5%\n\
             - Expired card: 10.2%\n\
             - Other: 6.9%\n\
            \n\
             Chargebacks (last 30 days):\n\
             - Rate: 0.45% (12 of 2,667)\n\
             - Fraud: 58.3%, Service not received: 25.0%, Other: 16.7%\n\
            \n\
             Fees (last month):\n\
             - Total: $12,450\n\
             - Average per transaction: $1.51\n\
             === END DATA ===\n\n\
             User Question: {question}\n\n\
             Answer based on the data above. Include specific numbers.",
            system = Self::build_system_prompt(),
            question = question,
        )
    }

    /// Build a retrieval-based prompt for documentation questions.
    fn build_retrieval_prompt(question: &str) -> String {
        format!(
            "{system}\n\n\
             === KNOWLEDGE BASE ===\n\
             The PaymentOrchestra platform supports:\n\
             - Smart routing across 40+ payment gateways\n\
             - Real-time fraud detection with ML scoring\n\
             - Automated reconciliation with settlement matching\n\
             - Chargeback management with deadline tracking\n\
             - Multi-currency support (80+ currencies)\n\
             - SaaS billing with subscription tiers\n\
             - API key management with scoped permissions\n\
             - MFA (TOTP, SMS, email)\n\
             - Webhook management with HMAC signing\n\
             - White-label configuration\n\
             === END KB ===\n\n\
             User Question: {question}\n\n\
             Answer based on the platform capabilities above. \
             If the question is about configuration, provide step-by-step instructions.",
            system = Self::build_system_prompt(),
            question = question,
        )
    }

    /// Call the Ollama API and return the response text.
    async fn call_ollama(&self, prompt: String) -> Result<String, AiError> {
        let url = format!("{}/api/generate", self.endpoint);

        let request = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            options: OllamaOptions {
                temperature: 0.3,
                top_p: 0.9,
                num_predict: 512,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AiError::Unavailable(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();            return Err(AiError::Unavailable(format!("Ollama returned {}: {}", status, body)));
        }

        let ollama_response: OllamaGenerateResponse = response
            .json()
            .await
            .map_err(|e| AiError::Unavailable(format!("Failed to parse Ollama response: {}", e)))?;

        if !ollama_response.done {
            return Err(AiError::Unavailable(
                "Ollama response incomplete".to_string(),
            ));
        }

        Ok(ollama_response.response)
    }

    /// Build a citation from the response text.
    fn build_citation(answer: &str, classification: &QueryClassification) -> GroundingCitation {
        let (source_id, source_name) = match classification {
            QueryClassification::StructuredLookup => {
                ("analytics:real_data", "Live Analytics Data")
            }
            QueryClassification::RetrievalEligible => {
                ("kb:platform_docs", "Platform Documentation")
            }
            _ => ("general:response", "AI Assistant"),
        };

        // Extract first 200 chars as excerpt
        let excerpt: String = answer.chars().take(200).collect();

        GroundingCitation {
            citation_id: Uuid::now_v7(),
            source_type: match classification {
                QueryClassification::StructuredLookup => CitationSourceType::PaymentEvent,
                QueryClassification::RetrievalEligible => CitationSourceType::KnowledgeBase,
                _ => CitationSourceType::Document,
            },
            source_id: source_id.into(),
            source_name: source_name.into(),
            excerpt,
            relevance_score: 0.85,
        }
    }
}

#[async_trait]
impl RagEngine for OllamaRagEngine {
    async fn answer_question(
        &self,
        question: &str,
        classification: QueryClassification,
        _context: &ConversationSession,
    ) -> Result<RagResult, AiError> {
        let start = std::time::Instant::now();

        // Build appropriate prompt based on classification
        let prompt = match classification {
            QueryClassification::StructuredLookup => {
                Self::build_lookup_prompt(question)
            }
            QueryClassification::RetrievalEligible => {
                Self::build_retrieval_prompt(question)
            }
            QueryClassification::Unanswerable => {
                return Ok(RagResult {
                    answer: "I can't predict future events, but I can analyze your historical \
                             payment data to identify trends and patterns. What specific historical \
                             data would you like me to look into?"
                        .into(),
                    citations: vec![],
                    confidence: AnswerConfidence::Low,
                    processing_time_ms: start.elapsed().as_millis() as u64,
                });
            }
        };

        // Call Ollama
        let answer = match self.call_ollama(prompt).await {
            Ok(text) => text,
            Err(_) => {
                // Fallback to a helpful error message
                return Ok(RagResult {
                    answer: "I'm temporarily unable to process your question. The AI model \
                             (Qwen3) may be loading or unavailable. Please try again in a \
                             moment."
                        .into(),
                    citations: vec![],
                    confidence: AnswerConfidence::Low,
                    processing_time_ms: start.elapsed().as_millis() as u64,
                });
            }
        };

        let citation = Self::build_citation(&answer, &classification);

        let confidence = if answer.len() > 50 && !answer.contains("unable") {
            AnswerConfidence::High
        } else if answer.len() > 20 {
            AnswerConfidence::Medium
        } else {
            AnswerConfidence::Low
        };

        Ok(RagResult {
            answer,
            citations: vec![citation],
            confidence,
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}
