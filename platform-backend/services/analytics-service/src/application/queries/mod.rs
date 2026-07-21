use uuid::Uuid;

use crate::domain::aggregates::PaymentAnalytics;
use crate::domain::entities::{
    AuthorizationEvent, ConnectorPerformance, DeclineBreakdownEntry, HourlyTrend, OperatorAnalytics,
    VolumeBucket,
};

pub struct PaymentAnalyticsQueryResult {
    pub analytics: PaymentAnalytics,
}

pub struct OperatorAnalyticsQueryResult {
    pub analytics: OperatorAnalytics,
}

pub struct VolumeTrendQueryResult {
    pub buckets: Vec<VolumeBucket>,
}

pub struct DeclineBreakdownQueryResult {
    pub breakdown: Vec<DeclineBreakdownEntry>,
}

pub struct ConnectorPerformanceQueryResult {
    pub connectors: Vec<ConnectorPerformance>,
}

pub struct HourlyTrendQueryResult {
    pub trends: Vec<HourlyTrend>,
}

pub struct AuthorizationEventsQueryResult {
    pub events: Vec<AuthorizationEvent>,
}
