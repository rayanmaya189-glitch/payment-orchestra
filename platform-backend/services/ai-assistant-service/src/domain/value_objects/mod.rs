#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryStatus {
    Processing,
    Completed,
    Failed,
}

impl QueryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationSource {
    Documentation,
    CodeExample,
    ApiReference,
    KnowledgeBase,
}

impl CitationSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Documentation => "documentation",
            Self::CodeExample => "code_example",
            Self::ApiReference => "api_reference",
            Self::KnowledgeBase => "knowledge_base",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "code_example" => Self::CodeExample,
            "api_reference" => Self::ApiReference,
            "knowledge_base" => Self::KnowledgeBase,
            _ => Self::Documentation,
        }
    }
}
