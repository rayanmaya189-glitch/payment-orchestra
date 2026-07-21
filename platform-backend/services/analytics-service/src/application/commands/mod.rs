use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GetPaymentAnalyticsCommand {
    pub operator_id: Uuid,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct GetOperatorAnalyticsCommand {
    pub operator_id: Uuid,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct GetVolumeTrendCommand {
    pub operator_id: Uuid,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct GetDeclineBreakdownCommand {
    pub operator_id: Uuid,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct GetConnectorPerformanceCommand {
    pub operator_id: Uuid,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct GetAuthorizationEventsCommand {
    pub operator_id: Uuid,
    pub start: String,
    pub end: String,
    pub limit: u32,
}
