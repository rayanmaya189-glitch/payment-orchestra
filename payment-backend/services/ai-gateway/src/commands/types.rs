//! Command types for AI Gateway

use uuid::Uuid;

pub struct ProcessAiQuery {
    pub operator_id: Uuid,
    pub question: String,
    pub has_attachment: bool,
}

pub struct RecordModelFailure;

pub struct RecordModelSuccess;

pub struct ResetQuota {
    pub operator_id: Uuid,
}
