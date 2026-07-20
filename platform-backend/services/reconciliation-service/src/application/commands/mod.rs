use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct PollSettlementCommand { pub operator_id: Uuid, pub connector_id: String, pub period_start: String, pub period_end: String }
#[derive(Debug, Clone)]
pub struct MatchSettlementCommand { pub batch_id: Uuid }
