use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PollSettlementCommand {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Clone)]
pub struct MatchSettlementCommand {
    pub batch_id: Uuid,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub exceptions: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct IngestSettlementCommand {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub raw_content: String,
    pub file_format: String,
    pub file_checksum: String,
}

#[derive(Debug, Clone)]
pub struct ListBatchesCommand {
    pub operator_id: Uuid,
    pub limit: u64,
    pub offset: u64,
}
