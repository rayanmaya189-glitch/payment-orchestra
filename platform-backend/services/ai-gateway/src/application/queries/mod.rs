use crate::domain::aggregates::{AiRequest, AiUsageStats};

pub struct RequestQueryResult {
    pub request: Option<AiRequest>,
}

pub struct RequestListQueryResult {
    pub requests: Vec<AiRequest>,
    pub total: u64,
}

pub struct UsageStatsQueryResult {
    pub stats: AiUsageStats,
}
