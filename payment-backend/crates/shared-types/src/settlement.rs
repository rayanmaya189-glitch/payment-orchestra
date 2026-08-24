use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementCycle {
    SameDay,
    NextDay,
    TwoDays,
    ThreeDays,
    Weekly,
    Custom(u32),
}

impl SettlementCycle {
    pub fn days(&self) -> u32 {
        match self {
            Self::SameDay => 0,
            Self::NextDay => 1,
            Self::TwoDays => 2,
            Self::ThreeDays => 3,
            Self::Weekly => 7,
            Self::Custom(d) => *d,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementExpectation {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub settlement_cycle: SettlementCycle,
    pub status: SettlementExpectationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementExpectationStatus {
    Pending,
    Settled,
    Overdue,
    Adjusted,
}
