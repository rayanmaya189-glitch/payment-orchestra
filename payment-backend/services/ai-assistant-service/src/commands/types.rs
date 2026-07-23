//! AI Assistant Service command types

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Command: StartConversation
// ---------------------------------------------------------------------------

pub struct StartConversation {
    pub operator_id: Uuid,
    pub title: String,
}

// ---------------------------------------------------------------------------
// Command: AskQuestion
// ---------------------------------------------------------------------------

pub struct AskQuestion {
    pub session_id: Uuid,
    pub operator_id: Uuid,
    pub question: String,
}
