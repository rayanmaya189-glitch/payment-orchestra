use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AiQueryCommand {
    pub principal_id: Uuid,
    pub query_text: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AiQueryResponse {
    pub query_id: Uuid,
    pub answer: String,
    pub citations: Vec<CitationResponse>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct CitationResponse {
    pub source_type: String,
    pub source_id: String,
    pub text_snippet: String,
    pub relevance_score: f64,
}
