use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct IngestSettlementRequest {
    pub acquirer_link_id: Uuid,
    pub file_format: String,
    // Note: raw_file would come from multipart upload
}

#[derive(Debug, Serialize)]
pub struct SettlementBatchResponse {
    pub settlement_batch_id: Uuid,
    pub status: String,
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub total_amount: i64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}
