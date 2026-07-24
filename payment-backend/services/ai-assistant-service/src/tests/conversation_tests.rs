//! Conversation tests: start, ask, history.

use crate::api::AiAssistantApi;
use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use crate::queries::{AiQueryHandler, QueryHandler};
use crate::repository::*;
use uuid::Uuid;

use super::setup;

#[tokio::test]
async fn test_start_conversation() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Auth Rate Analysis".into(),
        })
        .await
        .unwrap();

    assert_eq!(session.operator_id, operator_id);
    assert_eq!(session.title, "Auth Rate Analysis");
    assert!(session.messages.is_empty());
}

#[tokio::test]
async fn test_ask_question_with_citations() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Test Session".into(),
        })
        .await
        .unwrap();

    let result = pipeline
        .api
        .ask_question(AskQuestion {
            session_id: session.session_id,
            operator_id,
            question: "What was our authorization rate yesterday?".into(),
        })
        .await
        .unwrap();

    // AI-P-001: Grounding over fluency — must have citations
    assert!(
        !result.citations.is_empty(),
        "AI-P-001: Answer must include citations"
    );
    assert!(result.answer.contains("authorization rate"));
    assert_eq!(result.confidence, AnswerConfidence::High);
}

#[tokio::test]
async fn test_insufficient_grounding_returns_error() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Test Session".into(),
        })
        .await
        .unwrap();

    // Future/predictive questions should be unanswerable
    let result = pipeline
        .api
        .ask_question(AskQuestion {
            session_id: session.session_id,
            operator_id,
            question: "What will our revenue be next month?".into(),
        })
        .await;

    assert!(result.is_err(), "Predictive questions should be rejected");
    match result {
        Err(AiError::InsufficientGrounding) => {} // expected
        _ => panic!("Expected InsufficientGrounding error"),
    }
}

#[tokio::test]
async fn test_list_sessions() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Session 1".into(),
        })
        .await
        .unwrap();

    pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Session 2".into(),
        })
        .await
        .unwrap();

    let sessions = pipeline.api.list_sessions(operator_id).await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn test_session_info() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Info Test".into(),
        })
        .await
        .unwrap();

    let info = pipeline.api.get_session_info(session.session_id).await.unwrap();
    assert_eq!(info.title, "Info Test");
    assert_eq!(info.message_count, 0);
    assert_eq!(info.operator_id, operator_id);
    assert!(info.last_message_at.is_none());
}

#[tokio::test]
async fn test_health_status() {
    let pipeline = setup();
    let health = pipeline.api.health_status().await;
    assert!(health.available);
    assert!(health.uptime_hours == 0); // just started
}

#[tokio::test]
async fn test_session_not_found() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let result = pipeline
        .api
        .get_session(Uuid::now_v7(), operator_id)
        .await;

    assert!(result.is_err());
    match result {
        Err(AiError::SessionNotFound(_)) => {} // expected
        _ => panic!("Expected SessionNotFound error"),
    }
}
