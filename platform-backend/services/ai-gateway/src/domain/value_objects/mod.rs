#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

/// AI model types per SRS Part 6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    /// Qwen3 32B — primary reasoning / answer generation
    Reasoning,
    /// Qwen3-VL 8B — vision / document understanding / OCR
    Vision,
    /// BGE-M3 — dense + sparse + multi-vector embeddings
    Embedding,
    /// Cross-encoder reranker
    Reranker,
}

impl ModelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reasoning => "reasoning",
            Self::Vision => "vision",
            Self::Embedding => "embedding",
            Self::Reranker => "reranker",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "reasoning" => Ok(Self::Reasoning),
            "vision" => Ok(Self::Vision),
            "embedding" => Ok(Self::Embedding),
            "reranker" => Ok(Self::Reranker),
            _ => Err("unknown model type"),
        }
    }
}

/// Guardrail action taken on a request/response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailAction {
    pub action_type: GuardrailType,
    pub detail: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardrailType {
    InputBlocked,
    InputSanitized,
    OutputBlocked,
    CitationValidated,
    RateLimited,
}

impl GuardrailType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InputBlocked => "input_blocked",
            Self::InputSanitized => "input_sanitized",
            Self::OutputBlocked => "output_blocked",
            Self::CitationValidated => "citation_validated",
            Self::RateLimited => "rate_limited",
        }
    }
}

/// AI usage quota per operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuota {
    pub operator_id: uuid::Uuid,
    pub requests_today: u32,
    pub max_requests_per_day: u32,
    pub tokens_today: u64,
    pub max_tokens_per_day: u64,
}

impl UsageQuota {
    pub fn can_process(&self) -> bool {
        self.requests_today < self.max_requests_per_day && self.tokens_today < self.max_tokens_per_day
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_type_values() {
        assert_eq!(ModelType::Reasoning.as_str(), "reasoning");
        assert_eq!(ModelType::Vision.as_str(), "vision");
        assert_eq!(ModelType::Embedding.as_str(), "embedding");
    }

    #[test]
    fn test_model_type_from_str() {
        assert_eq!(ModelType::from_str("reasoning").unwrap(), ModelType::Reasoning);
        assert!(ModelType::from_str("unknown").is_err());
    }

    #[test]
    fn test_guardrail_type_values() {
        assert_eq!(GuardrailType::InputBlocked.as_str(), "input_blocked");
        assert_eq!(GuardrailType::OutputBlocked.as_str(), "output_blocked");
    }

    #[test]
    fn test_usage_quota_can_process() {
        let q = UsageQuota {
            operator_id: uuid::Uuid::now_v7(),
            requests_today: 50,
            max_requests_per_day: 100,
            tokens_today: 1000,
            max_tokens_per_day: 10000,
        };
        assert!(q.can_process());

        let q2 = UsageQuota {
            operator_id: uuid::Uuid::now_v7(),
            requests_today: 100,
            max_requests_per_day: 100,
            tokens_today: 1000,
            max_tokens_per_day: 10000,
        };
        assert!(!q2.can_process());
    }
}
