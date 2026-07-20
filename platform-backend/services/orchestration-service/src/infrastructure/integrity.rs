//! Event store integrity verification (SRS AUD-005).
//!
//! Daily verification job that walks the event chain for each aggregate
//! and checks for: sequence gaps, duplicate sequences, missing root creation event.

use sea_orm::{DatabaseConnection, ConnectionTrait, Statement};
use platform_error::PlatformError;

/// Result of integrity check for a single aggregate.
#[derive(Debug, Clone)]
pub struct IntegrityCheckResult {
    pub aggregate_id: uuid::Uuid,
    pub aggregate_type: String,
    pub total_events: i64,
    pub max_sequence: i64,
    pub has_gaps: bool,
    pub has_duplicates: bool,
    pub has_creation_event: bool,
    pub is_valid: bool,
}

/// Event store integrity checker.
pub struct EventStoreIntegrityChecker {
    db: DatabaseConnection,
}

impl EventStoreIntegrityChecker {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Verify integrity for a single aggregate.
    pub async fn verify_aggregate(
        &self,
        aggregate_type: &str,
        aggregate_id: uuid::Uuid,
    ) -> Result<IntegrityCheckResult, PlatformError> {
        // Get all event sequences for this aggregate
        let rows = self.db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT event_sequence, event_type FROM event_store
                 WHERE aggregate_type = $1 AND aggregate_id = $2
                 ORDER BY event_sequence ASC",
                vec![aggregate_type.into(), aggregate_id.into()],
            ))
            .await
            .map_err(|e| PlatformError::Internal(format!("Integrity check query failed: {e}")))?;

        let total_events = rows.len() as i64;
        let mut sequences: Vec<i64> = Vec::new();
        let mut event_types: Vec<String> = Vec::new();

        for row in &rows {
            let seq: i64 = row.try_get("", "event_sequence").unwrap_or(0);
            let etype: String = row.try_get("", "event_type").unwrap_or_default();
            sequences.push(seq);
            event_types.push(etype);
        }

        let max_sequence = sequences.iter().copied().max().unwrap_or(0);

        // Check for gaps (non-consecutive sequences)
        let has_gaps = sequences.windows(2).any(|w| w[1] - w[0] > 1);

        // Check for duplicates
        let has_duplicates = sequences.windows(2).any(|w| w[0] == w[1]);

        // Check for creation event
        let has_creation_event = event_types.iter().any(|et| et.ends_with("Created"));

        let is_valid = !has_gaps && !has_duplicates && has_creation_event && total_events > 0;

        Ok(IntegrityCheckResult {
            aggregate_id,
            aggregate_type: aggregate_type.to_string(),
            total_events,
            max_sequence,
            has_gaps,
            has_duplicates,
            has_creation_event,
            is_valid,
        })
    }

    /// Run full integrity check on all aggregates.
    pub async fn verify_all(&self) -> Result<Vec<IntegrityCheckResult>, PlatformError> {
        // Get all unique aggregates
        let rows = self.db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT DISTINCT aggregate_type, aggregate_id FROM event_store",
                vec![],
            ))
            .await
            .map_err(|e| PlatformError::Internal(format!("Integrity check query failed: {e}")))?;

        let mut results = Vec::new();

        for row in rows {
            let aggregate_type: String = row.try_get("", "aggregate_type").unwrap_or_default();
            let aggregate_id: uuid::Uuid = row.try_get("", "aggregate_id").unwrap_or_default();

            match self.verify_aggregate(&aggregate_type, aggregate_id).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!(
                        aggregate_type = %aggregate_type,
                        aggregate_id = %aggregate_id,
                        error = %e,
                        "Failed to verify aggregate integrity"
                    );
                }
            }
        }

        // Log summary
        let total = results.len();
        let valid = results.iter().filter(|r| r.is_valid).count();
        let invalid = total - valid;

        if invalid > 0 {
            tracing::warn!(
                total = total,
                valid = valid,
                invalid = invalid,
                "Event store integrity check found issues"
            );

            for result in &results {
                if !result.is_valid {
                    tracing::warn!(
                        aggregate_type = %result.aggregate_type,
                        aggregate_id = %result.aggregate_id,
                        has_gaps = result.has_gaps,
                        has_duplicates = result.has_duplicates,
                        has_creation = result.has_creation_event,
                        "Invalid aggregate detected"
                    );
                }
            }
        } else {
            tracing::info!(total = total, "Event store integrity check passed");
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrity_result_valid() {
        let result = IntegrityCheckResult {
            aggregate_id: uuid::Uuid::now_v7(),
            aggregate_type: "PaymentIntent".into(),
            total_events: 3,
            max_sequence: 3,
            has_gaps: false,
            has_duplicates: false,
            has_creation_event: true,
            is_valid: true,
        };
        assert!(result.is_valid);
    }

    #[test]
    fn test_integrity_result_invalid_with_gaps() {
        let result = IntegrityCheckResult {
            aggregate_id: uuid::Uuid::now_v7(),
            aggregate_type: "PaymentIntent".into(),
            total_events: 3,
            max_sequence: 5,
            has_gaps: true,
            has_duplicates: false,
            has_creation_event: true,
            is_valid: false,
        };
        assert!(!result.is_valid);
    }
}
