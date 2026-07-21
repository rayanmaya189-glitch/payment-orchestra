use axum::{extract::State, routing::post, Json, Router};
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::AiGatewayService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new().route("/ai/guard", post(process_request)).with_state(state)
}

async fn process_request(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let prompt = req["prompt"].as_str().unwrap_or("").to_string();
    if prompt.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "prompt is required", "code": "VALIDATION_ERROR"})),
        ));
    }

    let cmd = AiGatewayRequest {
        principal_id: auth.principal_id,
        prompt,
        model: req["model"].as_str().map(String::from),
    };

    match state.service.process(cmd).await {
        Ok(r) => Ok(Json(serde_json::json!({
            "request_id": r.request_id.to_string(),
            "blocked": r.blocked,
            "block_reason": r.block_reason,
            "redacted_prompt": r.redacted_prompt
        }))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string(), "code": "AI_REQUEST_FAILED"})),
        )),
    }
}
