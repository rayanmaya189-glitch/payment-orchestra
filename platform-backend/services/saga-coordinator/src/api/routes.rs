use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;
use crate::api::AppState;
use crate::api::dto::*;
use crate::application::commands::*;
use crate::application::services::SagaService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/sagas", post(start_saga).get(list_sagas))
        .route("/sagas/{saga_id}", get(get_saga))
        .route("/sagas/{saga_id}/advance", post(advance_saga))
        .route("/sagas/{saga_id}/fail", post(fail_saga))
        .route("/sagas/{saga_id}/compensate", post(compensate_saga))
        .route(
            "/sagas/{saga_id}/compensation/steps/{step_number}",
            post(mark_step_compensated),
        )
        .with_state(state)
}

/// Start a new saga. Supports both predefined payment lifecycle and custom steps.
async fn start_saga(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<StartSagaRequest>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = StartSagaCommand {
        saga_type: req.saga_type,
        steps: req
            .steps
            .unwrap_or_default()
            .into_iter()
            .map(|s| crate::domain::aggregates::SagaStepDef {
                name: s.name,
                service: s.service,
                action: s.action,
                compensation_action: s.compensation_action,
            })
            .collect(),
        payload: req.payload,
        started_by: Some(auth.principal_id),
    };

    match state.service.start(cmd).await {
        Ok(saga) => Ok((
            axum::http::StatusCode::CREATED,
            Json(serde_json::to_value(StartSagaResponse {
                saga_id: saga.saga_id,
                status: saga.status.as_str().to_string(),
                total_steps: saga.total_steps,
            })
            .unwrap()),
        )),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn get_saga(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = parse_uuid(&id)?;
    match state.service.get(sid).await {
        Ok(saga) => Ok(Json(serde_json::to_value(SagaDetailResponse::from(&saga)).unwrap())),
        Err(e) => Err(error_response(axum::http::StatusCode::NOT_FOUND, &e)),
    }
}

async fn list_sagas(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let query = ListSagasQuery {
        saga_type: params.get("saga_type").cloned(),
        status: params.get("status").cloned(),
        limit: params.get("limit").and_then(|v| v.parse().ok()),
    };

    match state.service.list(query).await {
        Ok(sagas) => {
            let responses: Vec<SagaDetailResponse> = sagas.iter().map(SagaDetailResponse::from).collect();
            Ok(Json(serde_json::json!({"sagas": responses})))
        }
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn advance_saga(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = parse_uuid(&id)?;
    match state
        .service
        .advance(AdvanceSagaCommand {
            saga_id: sid,
            step_result: None,
        })
        .await
    {
        Ok(step) => Ok(Json(serde_json::json!({
            "status": "advanced",
            "step": {
                "step_number": step.step_number,
                "name": step.name,
                "action": step.action,
            }
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn fail_saga(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<FailSagaRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = parse_uuid(&id)?;
    match state
        .service
        .fail(FailSagaCommand {
            saga_id: sid,
            error: req.error,
            error_code: req.error_code,
        })
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({"status": "failed"}))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn compensate_saga(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<CompensateSagaRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = parse_uuid(&id)?;
    match state
        .service
        .compensate(CompensateSagaCommand {
            saga_id: sid,
            reason: req.reason,
        })
        .await
    {
        Ok(steps) => {
            let step_responses: Vec<SagaStepResponse> = steps.iter().map(SagaStepResponse::from).collect();
            Ok(Json(serde_json::json!({
                "status": "compensating",
                "steps_to_compensate": step_responses,
            })))
        }
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn mark_step_compensated(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path((id, step_number)): Path<(String, u32)>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = parse_uuid(&id)?;
    match state
        .service
        .mark_step_compensated(MarkStepCompensatedCommand {
            saga_id: sid,
            step_number,
        })
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({"status": "compensated", "step_number": step_number}))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, (axum::http::StatusCode, Json<serde_json::Value>)> {
    Uuid::parse_str(s).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid UUID", "code": "INVALID_ID"})),
        )
    })
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
