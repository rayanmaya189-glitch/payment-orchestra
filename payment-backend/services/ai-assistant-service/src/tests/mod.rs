//! AI Assistant Service TDD tests
//!
//! Tests cover:
//! 1. Start a conversation session
//! 2. Ask a question → get answer with citations (AI-P-001)
//! 3. Insufficient grounding → error (AI-P-001)
//! 4. Cross-operator data isolation (AI-P-002)
//! 5. Output volume limit enforced (AI-EXFIL-001)
//! 6. Message length limit
//! 7. Rate limiting (AI-RATE-001)
//! 8. List sessions
//! 9. Session info
//! 10. Health status

use crate::api::AiAssistantApi;
use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use crate::queries::{AiQueryHandler, QueryHandler};
use crate::repository::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

fn setup() -> AiAssistantPipeline {
    AiAssistantPipeline::new()
}

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
async fn test_cross_operator_data_isolation() {
    let pipeline = setup();
    let operator_a = Uuid::now_v7();
    let operator_b = Uuid::now_v7();

    let session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id: operator_a,
            title: "A's Session".into(),
        })
        .await
        .unwrap();

    // Operator B tries to query A's session
    let result = pipeline
        .api
        .get_session(session.session_id, operator_b)
        .await;

    assert!(result.is_err(), "Cross-operator access should be denied");
    match result {
        Err(AiError::OperatorMismatch) => {} // expected
        _ => panic!("Expected OperatorMismatch error"),
    }
}

#[tokio::test]
async fn test_output_volume_limit_enforced() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let mut session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Volume Test".into(),
        })
        .await
        .unwrap();

    // Add more than MAX_OUTPUT_RECORDS messages
    for i in 0..MAX_OUTPUT_RECORDS + 10 {
        session.messages.push(ConversationMessage {
            message_id: Uuid::now_v7(),
            role: MessageRole::User,
            content: format!("Query number {}", i),
            citations: Vec::new(),
            created_at: chrono::Utc::now(),
        });
    }
    pipeline
        .session_repo
        .write()
        .await
        .save(&session)
        .await
        .unwrap();

    // Searching with a broad term should hit the volume limit
    let result = pipeline
        .api
        .search_session_history(session.session_id, operator_id, "Query")
        .await;

    assert!(result.is_err(), "Volume limit should be enforced");
    match result {
        Err(AiError::QueryTooBroad { .. }) => {} // expected
        _ => panic!("Expected QueryTooBroad error"),
    }
}

#[tokio::test]
async fn test_message_length_limit() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "Length Test".into(),
        })
        .await
        .unwrap();

    // Create a message longer than MAX_QUESTION_LENGTH
    let long_question = "a".repeat(MAX_QUESTION_LENGTH + 1);

    let result = pipeline
        .api
        .ask_question(AskQuestion {
            session_id: session.session_id,
            operator_id,
            question: long_question,
        })
        .await;

    assert!(result.is_err(), "Overly long messages should be rejected");
    match result {
        Err(AiError::MessageTooLong { .. }) => {} // expected
        _ => panic!("Expected MessageTooLong error"),
    }
}

#[tokio::test]
async fn test_rate_limit_exceeded() {
    // Use a strict rate limiter for testing
    let session_repo = std::sync::Arc::new(tokio::sync::RwLock::new(
        InMemoryConversationSessionRepository::new(),
    ));
    let adapter = crate::pipeline::ArcRepoAdapter(session_repo.clone());
    let rate_limiter: Box<dyn RateLimiter> = Box::new(TokenBucketRateLimiter::new(2)); // max 2 per minute
    let rag_engine: Box<dyn RagEngine> = Box::new(SimulatedRagEngine);

    let ch: Box<dyn CommandHandler> = Box::new(AiCommandHandler::new(
        adapter.clone(),
        rag_engine,
        rate_limiter,
    ));
    let qh: Box<dyn QueryHandler> = Box::new(AiQueryHandler::new(adapter));
    let api = crate::api::AiAssistantApi::new(ch, qh);

    let operator_id = Uuid::now_v7();
    let session = api
        .start_conversation(StartConversation {
            operator_id,
            title: "Rate Limit Test".into(),
        })
        .await
        .unwrap();

    // First two should succeed
    api.ask_question(AskQuestion {
        session_id: session.session_id,
        operator_id,
        question: "What is our auth rate?".into(),
    })
    .await
    .unwrap();

    api.ask_question(AskQuestion {
        session_id: session.session_id,
        operator_id,
        question: "Tell me about declines".into(),
    })
    .await
    .unwrap();

    // Third should fail (rate limit at 2)
    let result = api
        .ask_question(AskQuestion {
            session_id: session.session_id,
            operator_id,
            question: "How do I set up a refund?".into(),
        })
        .await;

    assert!(result.is_err(), "Rate limit should be exceeded");
    match result {
        Err(AiError::RateLimitExceeded(_)) => {} // expected
        _ => panic!("Expected RateLimitExceeded error"),
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

#[tokio::test]
async fn test_rag_answer_for_howto_question() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    let session = pipeline
        .api
        .start_conversation(StartConversation {
            operator_id,
            title: "How-to Test".into(),
        })
        .await
        .unwrap();

    let result = pipeline
        .api
        .ask_question(AskQuestion {
            session_id: session.session_id,
            operator_id,
            question: "How do I set up a refund?".into(),
        })
        .await
        .unwrap();

    assert!(!result.citations.is_empty());
    assert_eq!(result.confidence, AnswerConfidence::High);
}
