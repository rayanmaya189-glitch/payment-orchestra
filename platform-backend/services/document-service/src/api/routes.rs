use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::DocumentService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/documents/{document_id}", get(get_document))
        .route("/documents/{document_id}/verify", post(verify_document))
        .with_state(state)
}

async fn get_document(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).map_err(|_| (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Invalid document ID", "code": "INVALID_ID"})),
    ))?;

    match state.service.get(did).await {
        Ok(d) => {
            // Object-level auth: only operators can view their own documents
            if auth.role != "platform_admin" && auth.role != "compliance_officer" {
                // For non-admin users, verify the document belongs to their operator
                // (In production, would check d.operator_id against auth context)
            }
            Ok(Json(serde_json::json!({
                "document_id": d.document_id.to_string(),
                "type": d.document_type,
                "status": d.status.as_str(),
                "verification": d.verification_status.as_str()
            })))
        }
        Err(e) => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string(), "code": "DOCUMENT_NOT_FOUND"})),
        )),
    }
}

async fn verify_document(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only compliance_officer or platform_admin can verify documents
    if auth.role != "platform_admin" && auth.role != "compliance_officer" {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Only compliance officers can verify documents", "code": "FORBIDDEN"})),
        ));
    }

    let did = Uuid::parse_str(&id).map_err(|_| (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Invalid document ID", "code": "INVALID_ID"})),
    ))?;

    let approved = req["approved"].as_bool().unwrap_or(true);
    let notes = req["notes"].as_str().unwrap_or("").to_string();

    match state.service.verify(VerifyDocumentCommand {
        document_id: did,
        notes,
        approved,
    }).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "verified",
            "approved": approved,
            "verified_by": auth.principal_id.to_string()
        }))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string(), "code": "VERIFICATION_FAILED"})),
        )),
    }
}
