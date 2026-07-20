use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{ModelType, GuardrailAction};

/// AI Gateway request — routes to appropriate model pool.
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub request_id: Uuid,
    pub operator_id: Uuid,
    pub query: String,
    pub has_image: bool,
    pub model_preference: Option<ModelType>,
    pub created_at: DateTime<Utc>,
}

impl AiRequest {
    pub fn new(operator_id: Uuid, query: String, has_image: bool) -> Self {
        Self {
            request_id: Uuid::now_v7(),
            operator_id,
            query,
            has_image,
            model_preference: None,
            created_at: Utc::now(),
        }
    }

    /// Select the appropriate model based on request characteristics.
    pub fn select_model(&self) -> ModelType {
        if let Some(pref) = &self.model_preference {
            return pref.clone();
        }
        if self.has_image {
            ModelType::Vision
        } else {
            ModelType::Reasoning
        }
    }
}

/// AI Gateway response — validated and guarded.
#[derive(Debug, Clone)]
pub struct AiResponse {
    pub request_id: Uuid,
    pub content: String,
    pub model_used: ModelType,
    pub citations: Vec<String>,
    pub guardrail_actions: Vec<GuardrailAction>,
    pub latency_ms: u64,
    pub created_at: DateTime<Utc>,
}

impl AiResponse {
    pub fn new(request_id: Uuid, content: String, model_used: ModelType) -> Self {
        Self {
            request_id,
            content,
            model_used,
            citations: Vec::new(),
            guardrail_actions: Vec::new(),
            latency_ms: 0,
            created_at: Utc::now(),
        }
    }
}

/// AI Gateway circuit breaker state per model.
#[derive(Debug, Clone)]
pub struct ModelCircuitBreaker {
    pub model_type: ModelType,
    pub is_open: bool,
    pub failure_count: u32,
    pub last_failure_at: Option<DateTime<Utc>>,
}

impl ModelCircuitBreaker {
    pub fn new(model_type: ModelType) -> Self {
        Self {
            model_type,
            is_open: false,
            failure_count: 0,
            last_failure_at: None,
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_at = Some(Utc::now());
        if self.failure_count >= 5 {
            self.is_open = true;
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.is_open = false;
    }

    pub fn allow_request(&self) -> bool {
        !self.is_open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_request() {
        let req = AiRequest::new(Uuid::now_v7(), "What is my balance?".into(), false);
        assert_eq!(req.query, "What is my balance?");
        assert!(!req.has_image);
    }

    #[test]
    fn test_select_model_text() {
        let req = AiRequest::new(Uuid::now_v7(), "query".into(), false);
        assert_eq!(req.select_model(), ModelType::Reasoning);
    }

    #[test]
    fn test_select_model_image() {
        let req = AiRequest::new(Uuid::now_v7(), "query".into(), true);
        assert_eq!(req.select_model(), ModelType::Vision);
    }

    #[test]
    fn test_select_model_preference() {
        let mut req = AiRequest::new(Uuid::now_v7(), "query".into(), true);
        req.model_preference = Some(ModelType::Embedding);
        assert_eq!(req.select_model(), ModelType::Embedding);
    }

    #[test]
    fn test_circuit_breaker() {
        let mut cb = ModelCircuitBreaker::new(ModelType::Reasoning);
        assert!(cb.allow_request());

        for _ in 0..5 {
            cb.record_failure();
        }
        assert!(!cb.allow_request());

        cb.record_success();
        assert!(cb.allow_request());
    }
}
