//! gRPC service implementation for the outbox relay management API.
//! Translates between protobuf types and domain types.
//! Provides operational RPCs for monitoring the transactional outbox relay.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::AppendEntry;
use crate::api::OutboxRelayApi;
use crate::domain::OutboxRelayError;

use platform_proto::outbox_relay::outbox_relay_service_server::OutboxRelayService;
use platform_proto::outbox_relay::*;

pub struct OutboxRelayGrpcService {
    api: OutboxRelayApi,
}

impl OutboxRelayGrpcService {
    pub fn new(api: OutboxRelayApi) -> Self {
        Self { api }
    }
}

#[tonic::async_trait]
impl OutboxRelayService for OutboxRelayGrpcService {
    async fn get_metrics(
        &self,
        _request: Request<GetMetricsRequest>,
    ) -> Result<Response<RelayMetricsView>, Status> {
        match self.api.get_metrics().await {
            Ok(metrics) => Ok(Response::new(RelayMetricsView {
                total_polled: metrics.total_polled,
                total_published: metrics.total_published,
                total_failed: metrics.total_failed,
                total_duplicates_skipped: metrics.total_duplicates_skipped,
                last_polled_at: metrics.last_polled_at.map(|t| t.to_rfc3339()),
                queue_depth: metrics.queue_depth,
                is_running: metrics.is_running,
            })),
            Err(e) => Err(outbox_error_to_status(e)),
        }
    }

    async fn get_entry(
        &self,
        request: Request<GetEntryRequest>,
    ) -> Result<Response<OutboxEntryView>, Status> {
        let req = request.into_inner();
        let entry_id = parse_uuid(&req.entry_id, "entry_id")?;

        match self.api.get_entry(entry_id).await {
            Ok(entry) => Ok(Response::new(entry_to_view(entry))),
            Err(e) => Err(outbox_error_to_status(e)),
        }
    }

    async fn list_unpublished(
        &self,
        request: Request<ListUnpublishedRequest>,
    ) -> Result<Response<ListUnpublishedResponse>, Status> {
        let req = request.into_inner();
        // batch_size from request is advisory; API returns all unpublished
        match self.api.list_unpublished().await {
            Ok(entries) => {
                let batch = if req.batch_size > 0 {
                    entries.into_iter().take(req.batch_size as usize).collect()
                } else {
                    entries
                };
                Ok(Response::new(ListUnpublishedResponse {
                    entries: batch.into_iter().map(entry_to_view).collect(),
                }))
            }
            Err(e) => Err(outbox_error_to_status(e)),
        }
    }

    async fn append_entry(
        &self,
        request: Request<AppendEntryRequest>,
    ) -> Result<Response<OutboxEntryView>, Status> {
        let req = request.into_inner();
        let aggregate_id = parse_uuid(&req.aggregate_id, "aggregate_id")?;

        let cmd = AppendEntry {
            aggregate_type: req.aggregate_type,
            aggregate_id,
            event_type: req.event_type,
            event_version: req.event_version as i16,
            payload: req.payload,
        };

        match self.api.append_entry(cmd).await {
            Ok(entry) => Ok(Response::new(entry_to_view(entry))),
            Err(e) => Err(outbox_error_to_status(e)),
        }
    }
}

// ─── View Helpers ───────────────────────────────────────────────────────────

fn entry_to_view(entry: crate::domain::OutboxEntry) -> OutboxEntryView {
    OutboxEntryView {
        entry_id: entry.outbox_id.to_string(),
        aggregate_type: entry.aggregate_type,
        aggregate_id: entry.aggregate_id.to_string(),
        event_type: entry.event_type,
        event_version: entry.event_version as i32,
        payload: entry.payload,
        created_at: entry.created_at.to_rfc3339(),
        published_at: entry.published_at.map(|t| t.to_rfc3339()),
    }
}

// ─── Error Conversion ───────────────────────────────────────────────────────

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn outbox_error_to_status(e: OutboxRelayError) -> Status {
    match e {
        OutboxRelayError::EntryNotFound(id) => Status::not_found(format!("Entry not found: {}", id)),
        OutboxRelayError::PublishFailed(ref msg) => Status::internal(format!("Publish failed: {}", msg)),
        OutboxRelayError::MaxRetriesExceeded(ref n) => Status::internal(format!("Max retries ({}) exceeded", n)),
        OutboxRelayError::NotRunning => Status::failed_precondition("Relay is not running"),
        OutboxRelayError::AlreadyRunning => Status::failed_precondition("Relay is already running"),
        OutboxRelayError::DatabaseError(ref msg) => Status::internal(format!("Database error: {}", msg)),
    }
}

impl From<OutboxRelayError> for Status {
    fn from(e: OutboxRelayError) -> Self {
        outbox_error_to_status(e)
    }
}
