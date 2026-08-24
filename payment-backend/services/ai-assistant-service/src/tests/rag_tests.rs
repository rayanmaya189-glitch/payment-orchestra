//! RAG tests: RAG responses, grounding.

use crate::commands::*;
use crate::domain::*;
use uuid::Uuid;

use super::setup;

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
