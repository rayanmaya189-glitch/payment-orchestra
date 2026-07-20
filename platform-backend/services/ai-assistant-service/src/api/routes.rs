use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::AiAssistantService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ai/query", post(query_ai))
        .route("/ai/queries/{query_id}", get(get_query))
        .with_state(state)
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
    session_id: Option<String>,
}

#[derive(Serialize)]
struct QueryResponseJson {
    query_id: String,
    answer: String,
    citations: Vec<CitationJson>,
    confidence: f64,
}

#[derive(Serialize)]
struct CitationJson {
    source_type: String,
    source_id: String,
    text_snippet: String,
    relevance_score: f64,
}

async fn query_ai(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponseJson>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = AiQueryCommand {
        principal_id: Uuid::nil(),
        query_text: req.query,
        session_id: req.session_id,
    };

    match state.service.query(cmd).await {
        Ok(resp) => Ok(Json(QueryResponseJson {
            query_id: resp.query_id.to_string(),
            answer: resp.answer,
            citations: resp.citations.iter().map(|c| CitationJson {
                source_type: c.source_type.clone(),
                source_id: c.source_id.clone(),
                text_snippet: c.text_snippet.clone(),
                relevance_score: c.relevance_score,
            }).collect(),
            confidence: resp.confidence,
        })),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn get_query(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok(Json(serde_json::json!({"status": "ok"})))
}
