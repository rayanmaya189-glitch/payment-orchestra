use std::fmt;

#[derive(Debug, Clone)]
pub struct CostUsd(pub f64);

impl CostUsd {
    pub fn new(amount: f64) -> Result<Self, String> {
        if amount < 0.0 {
            return Err("Cost cannot be negative".into());
        }
        if !amount.is_finite() {
            return Err("Cost must be a finite number".into());
        }
        Ok(Self(amount))
    }

    pub fn zero() -> Self {
        Self(0.0)
    }

    pub fn add(self, other: CostUsd) -> CostUsd {
        CostUsd(self.0 + other.0)
    }

    pub fn as_minor_units(&self) -> i64 {
        (self.0 * 100.0).round() as i64
    }
}

impl fmt::Display for CostUsd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.6}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct TokenCount(pub u32);

impl TokenCount {
    pub fn new(count: u32) -> Self {
        Self(count)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn add(self, other: TokenCount) -> TokenCount {
        TokenCount(self.0 + other.0)
    }

    pub fn exceeds_limit(&self, limit: u32) -> bool {
        self.0 > limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptCategory {
    Safe,
    InjectionAttempt,
    PiiDetected,
    HarmfulContent,
    TooLong,
    Empty,
}

impl PromptCategory {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::InjectionAttempt | Self::HarmfulContent | Self::Empty | Self::TooLong)
    }
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub model: String,
    pub input_cost_per_1k_tokens: f64,
    pub output_cost_per_1k_tokens: f64,
    pub max_context_tokens: u32,
}

impl ModelPricing {
    pub fn qwen3() -> Self {
        Self {
            model: "qwen3".into(),
            input_cost_per_1k_tokens: 0.0001,
            output_cost_per_1k_tokens: 0.0002,
            max_context_tokens: 32_768,
        }
    }

    pub fn qwen3_72b() -> Self {
        Self {
            model: "qwen3-72b".into(),
            input_cost_per_1k_tokens: 0.0005,
            output_cost_per_1k_tokens: 0.001,
            max_context_tokens: 131_072,
        }
    }

    pub fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> CostUsd {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k_tokens;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k_tokens;
        CostUsd(input_cost + output_cost)
    }

    pub fn for_model(model: &str) -> Self {
        match model {
            "qwen3-72b" => Self::qwen3_72b(),
            _ => Self::qwen3(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_usd_creation() {
        assert!(CostUsd::new(1.5).is_ok());
        assert!(CostUsd::new(0.0).is_ok());
        assert!(CostUsd::new(-1.0).is_err());
        assert!(CostUsd::new(f64::NAN).is_err());
    }

    #[test]
    fn test_cost_addition() {
        let c1 = CostUsd(0.5);
        let c2 = CostUsd(0.3);
        let result = c1.add(c2);
        assert!((result.0 - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_token_count() {
        let t = TokenCount::new(100);
        assert!(!t.exceeds_limit(200));
        assert!(t.exceeds_limit(50));
        let combined = t.add(TokenCount::new(150));
        assert_eq!(combined.0, 250);
    }

    #[test]
    fn test_model_pricing_estimate() {
        let pricing = ModelPricing::qwen3();
        let cost = pricing.estimate_cost(1000, 500);
        assert!((cost.0 - 0.0002).abs() < 0.00001);
    }

    #[test]
    fn test_prompt_category_blocking() {
        assert!(PromptCategory::Safe.is_blocked() == false);
        assert!(PromptCategory::InjectionAttempt.is_blocked());
        assert!(PromptCategory::HarmfulContent.is_blocked());
    }
}
