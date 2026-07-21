//! Mock LLM provider for testing.
//!
//! Returns canned responses based on keyword matching in the user's query.
//! Useful for integration tests and development without a running Ollama instance.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::rules::LlmProvider;
use platform_error::PlatformError;

/// Mock LLM provider that returns canned responses.
///
/// Populate with keyword-to-response mappings. When a query matches a keyword,
/// the corresponding response is returned. Falls back to a default response.
pub struct MockLlmProvider {
    /// Keyword -> canned response mapping.
    responses: HashMap<String, String>,
    /// Default response when no keyword matches.
    default_response: String,
}

impl MockLlmProvider {
    /// Create a mock provider with a custom default response.
    pub fn new(default_response: String) -> Self {
        Self {
            responses: HashMap::new(),
            default_response,
        }
    }

    /// Create a mock provider with pre-populated responses for common queries.
    pub fn with_defaults() -> Self {
        let mut responses = HashMap::new();
        responses.insert(
            "routing".to_string(),
            "Payment routing determines which payment connector processes each transaction \
             based on rules like geography, cost, success rates, and currency."
                .to_string(),
        );
        responses.insert(
            "authorization".to_string(),
            "Authorization rates measure the percentage of payment attempts that are \
             successfully approved by the issuing bank."
                .to_string(),
        );
        responses.insert(
            "connector".to_string(),
            "Connectors are payment service providers (Stripe, Adyen, etc.) that process \
             transactions. Each has different capabilities, fees, and success rates."
                .to_string(),
        );
        responses.insert(
            "compliance".to_string(),
            "The platform supports KYC/AML checks, PCI DSS compliance, and regulatory \
             reporting through the compliance service."
                .to_string(),
        );

        Self {
            responses,
            default_response: "I'm an AI assistant for a payment orchestration platform. \
                Please ask me about payments, connectors, routing, or compliance."
                .to_string(),
        }
    }

    /// Add or override a canned response for a keyword.
    pub fn add_response(&mut self, keyword: String, response: String) {
        self.responses.insert(keyword, response);
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        messages: Vec<(&str, &str)>,
        _model: &str,
    ) -> Result<String, PlatformError> {
        // Find the last user message
        let user_message = messages
            .iter()
            .rev()
            .find(|(role, _)| *role == "user")
            .map(|(_, content)| *content)
            .unwrap_or("");

        let query_lower = user_message.to_lowercase();

        // Find matching keyword response
        let response = self
            .responses
            .iter()
            .find(|(keyword, _)| query_lower.contains(keyword.as_str()))
            .map(|(_, response)| response.clone())
            .unwrap_or_else(|| self.default_response.clone());

        tracing::debug!(
            query_len = user_message.len(),
            response_len = response.len(),
            "Mock LLM provider returning canned response"
        );

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_returns_keyword_match() {
        let mock = MockLlmProvider::with_defaults();
        let messages = vec![("user", "What is payment routing?")];
        let response = mock.complete(messages, "qwen3").await.unwrap();
        assert!(response.contains("routing"));
        assert!(response.contains("connector"));
    }

    #[tokio::test]
    async fn test_mock_returns_default() {
        let mock = MockLlmProvider::new("default answer".to_string());
        let messages = vec![("user", "xyzzy unknown question")];
        let response = mock.complete(messages, "qwen3").await.unwrap();
        assert_eq!(response, "default answer");
    }

    #[tokio::test]
    async fn test_mock_custom_response() {
        let mut mock = MockLlmProvider::new("fallback".to_string());
        mock.add_response("test".to_string(), "custom answer".to_string());
        let messages = vec![("user", "tell me about test data")];
        let response = mock.complete(messages, "qwen3").await.unwrap();
        assert_eq!(response, "custom answer");
    }

    #[tokio::test]
    async fn test_mock_ignores_system_messages() {
        let mock = MockLlmProvider::with_defaults();
        let messages = vec![("system", "You are helpful"), ("user", "authorization rates")];
        let response = mock.complete(messages, "qwen3").await.unwrap();
        assert!(response.contains("Authorization"));
    }
}
