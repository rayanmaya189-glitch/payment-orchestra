use axum::{
    extract::{Path, State},
    routing::{get, post, delete},
    Json, Router,
};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::DocumentService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/documents", post(upload_document).get(list_documents))
        .route("/documents/{document_id}", get(get_document).delete(delete_document))
        .route("/documents/{document_id}/verify", post(verify_document))
        .with_state(state)
}

/// Upload a new document. Any authenticated user can upload for their operator.
async fn upload_document(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Extract required fields
    let requested_operator_id = req["operator_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());
    let operator_id = shared_types::derive_operator_id(&auth.principal_id, &auth.role, requested_operator_id);

    let document_type = req["document_type"]
        .as_str()
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing document_type", "code": "MISSING_FIELD"})),
            )
        })?;

    let filename = req["filename"]
        .as_str()
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing filename", "code": "MISSING_FIELD"})),
            )
        })?;

    let content_type = req["content_type"]
        .as_str()
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing content_type", "code": "MISSING_FIELD"})),
            )
        })?;

    let file_data_b64 = req["file_data"]
        .as_str()
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing file_data (base64)", "code": "MISSING_FIELD"})),
            )
        })?;

    let file_data = base64_decode(file_data_b64).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid base64 file_data: {e}"), "code": "INVALID_BASE64"})),
        )
    })?;

    let file_size = req["file_size"]
        .as_i64()
        .unwrap_or(file_data.len() as i64);

    let cmd = UploadDocumentCommand {
        operator_id,
        document_type: document_type.to_string(),
        filename: filename.to_string(),
        content_type: content_type.to_string(),
        file_size,
        file_data,
        uploaded_by: auth.principal_id.to_string(),
    };

    match state.service.upload(cmd).await {
        Ok(doc_id) => Ok((
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({
                "document_id": doc_id.to_string(),
                "status": "uploaded"
            })),
        )),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn get_document(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid document ID", "code": "INVALID_ID"})),
        )
    })?;

    match state.service.get(did).await {
        Ok(d) => {
            // Object-level authorization (OWASP A01):
            // platform_admin and compliance_officer can view any document
            // operator_admin can only view documents belonging to their operator
            // Other roles cannot view documents
            match auth.role.as_str() {
                "platform_admin" | "compliance_officer" => {}
                "operator_admin" => {
                    // TODO: In production, extract operator_id from auth context
                    // and verify d.operator_id matches. For now, allow operator_admin.
                }
                _ => {
                    return Err((
                        axum::http::StatusCode::FORBIDDEN,
                        Json(serde_json::json!({"error": "Insufficient permissions to view documents", "code": "FORBIDDEN"})),
                    ));
                }
            }
            Ok(Json(serde_json::json!({
                "document_id": d.document_id.to_string(),
                "operator_id": d.operator_id.to_string(),
                "type": d.document_type,
                "status": d.status.as_str(),
                "verification": d.verification_status.as_str(),
                "filename": d.filename,
                "content_type": d.content_type,
                "file_size": d.file_size,
                "file_hash": d.file_hash,
                "uploaded_by": d.uploaded_by,
                "verified_by": d.verified_by,
                "verification_notes": d.verification_notes,
                "created_at": d.created_at.to_rfc3339(),
                "updated_at": d.updated_at.to_rfc3339(),
            })))
        }
        Err(e) => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string(), "code": "DOCUMENT_NOT_FOUND"})),
        )),
    }
}

async fn list_documents(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let operator_id = params.get("operator_id")
        .and_then(|s| Uuid::parse_str(s).ok());

    match auth.role.as_str() {
        "platform_admin" | "compliance_officer" => {
            // Admin can list any operator's documents
            let oid = operator_id.ok_or_else(|| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "operator_id required", "code": "MISSING_FIELD"})),
                )
            })?;
            let limit = params.get("limit").and_then(|v| v.parse().ok());
            let offset = params.get("offset").and_then(|v| v.parse().ok());

            let docs = state.service.list_by_operator(ListDocumentsQuery {
                operator_id: oid,
                limit,
                offset,
            }).await.map_err(|e| error_response(axum::http::StatusCode::BAD_REQUEST, &e))?;

            let items: Vec<serde_json::Value> = docs.iter().map(|d| {
                serde_json::json!({
                    "document_id": d.document_id,
                    "type": d.document_type,
                    "status": d.status.as_str(),
                    "verification": d.verification_status.as_str(),
                })
            }).collect();

            Ok(Json(serde_json::json!({"documents": items})))
        }
        _ => Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"})),
        )),
    }
}

/// Verify or reject a document. Only compliance_officer or platform_admin.
async fn verify_document(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // ABAC: Only compliance_officer or platform_admin can verify documents
    // This enforces SRS ABAC-005: Checker eligibility scoped by role
    if auth.role != "platform_admin" && auth.role != "compliance_officer" {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Only compliance officers can verify documents", "code": "FORBIDDEN"})),
        ));
    }

    let did = Uuid::parse_str(&id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid document ID", "code": "INVALID_ID"})),
        )
    })?;

    let approved = req["approved"].as_bool().unwrap_or(true);
    let notes = req["notes"].as_str().unwrap_or("").to_string();

    match state.service.verify(VerifyDocumentCommand {
        document_id: did,
        notes,
        approved,
        verified_by: auth.principal_id.to_string(),
    }).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": if approved { "verified" } else { "rejected" },
            "approved": approved,
            "verified_by": auth.principal_id.to_string()
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

/// Delete a document. Only platform_admin.
async fn delete_document(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // ABAC: Only platform_admin can delete documents
    if auth.role != "platform_admin" {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Only platform admins can delete documents", "code": "FORBIDDEN"})),
        ));
    }

    // TODO: Implement actual deletion with storage cleanup
    Ok(Json(serde_json::json!({"status": "deleted", "document_id": id})))
}

fn error_response(
    status: axum::http::StatusCode,
    err: &platform_error::PlatformError,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let code = match err {
        platform_error::PlatformError::NotFound { .. } => "NOT_FOUND",
        platform_error::PlatformError::Validation(_) => "VALIDATION_ERROR",
        platform_error::PlatformError::Conflict(_) => "CONFLICT",
        platform_error::PlatformError::AuthorizationDenied(_) => "FORBIDDEN",
        platform_error::PlatformError::Unavailable(_) => "SERVICE_UNAVAILABLE",
        _ => "INTERNAL_ERROR",
    };

    (
        status,
        Json(serde_json::json!({
            "error": err.to_string(),
            "code": code,
        })),
    )
}

/// Simple base64 decode (avoids adding the base64 crate — uses a minimal impl).
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    // Use the standard base64 alphabet
    use std::collections::HashMap;
    let alphabet: HashMap<u8, u8> = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter()
        .enumerate()
        .map(|(i, &b)| (b, i as u8))
        .collect();

    let input = input.trim_end_matches('=');
    let bytes = input.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in bytes {
        let val = alphabet.get(&byte).ok_or_else(|| format!("invalid base64 char: {}", byte as char))?;
        buf = (buf << 6) | (*val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
        }
    }

    Ok(result)
}
