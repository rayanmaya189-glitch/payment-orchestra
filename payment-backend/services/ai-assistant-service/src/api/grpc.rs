//! gRPC service implementation for ai-assistant-service (BC-12).
//! Translates between protobuf types and domain types for AI Q&A lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::{self, AiError};
use crate::queries::QueryHandler;
use crate::repository::ConversationSessionRepository;

use platform_proto::ai_assistant::ai_assistant_service_server::AiAssistantService;
use platform_proto::ai_assistant::*;

pub struct AiAssistantGrpcService<C, Q, R> {
    commands: C,
    queries: Q,
    repo: R,
}

impl<C, Q, R> AiAssistantGrpcService<C, Q, R> {
    pub fn new(commands: C, queries: Q, repo: R) -> Self {
        Self { commands, queries, repo }
    }
}

#[tonic::async_trait]
impl<C, Q, R> AiAssistantService for AiAssistantGrpcService<C, Q, R>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
    R: ConversationSessionRepository + Send + Sync + 'static,
{
    async fn ask_assistant(
        &self,
        request: Request<AskAssistantRequest>,
    ) -> Result<Response<AskAssistantResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        // If no session_id provided, start a new conversation
        let session_id = if req.session_id.is_empty() {
            let start_cmd = commands::StartConversation {
                operator_id,
                title: req.query.chars().take(100).collect(),
            };
            match self.commands.start_conversation(start_cmd).await {
                Ok(session) => session.session_id,
                Err(e) => return Err(ai_error_to_status(e)),
            }
        } else {
            parse_uuid(&req.session_id, "session_id")?
        };

        let cmd = commands::AskQuestion {
            session_id,
            operator_id,
            question: req.query,
        };

        match self.commands.ask_question(cmd).await {
            Ok(result) => {
                let citations: Vec<Citation> = result
                    .citations
                    .into_iter()
                    .map(|c| Citation {
                        source: c.source_name,
                        snippet: c.excerpt,
                        relevance_score: c.relevance_score,
                    })
                    .collect();

                let requires_human = result.confidence == domain::AnswerConfidence::Low
                    || result.confidence == domain::AnswerConfidence::InsufficientData;

                Ok(Response::new(AskAssistantResponse {
                    session_id: session_id.to_string(),
                    answer: result.answer,
                    citations,
                    requires_human_action: requires_human,
                    suggested_action: String::new(),
                    suggested_action_params: String::new(),
                }))
            }
            Err(e) => Err(ai_error_to_status(e)),
        }
    }

    async fn get_session_history(
        &self,
        request: Request<GetSessionHistoryRequest>,
    ) -> Result<Response<GetSessionHistoryResponse>, Status> {
        let req = request.into_inner();
        let session_id = parse_uuid(&req.session_id, "session_id")?;
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        match self.queries.get_session(session_id, operator_id).await {
            Ok(session) => {
                let proto_messages: Vec<proto_ConversationMessage> = session
                    .messages
                    .into_iter()
                    .map(|msg| {
                        let role = match msg.role {
                            domain::MessageRole::User => "user".to_string(),
                            domain::MessageRole::Assistant => "assistant".to_string(),
                        };
                        let citations: Vec<Citation> = msg
                            .citations
                            .into_iter()
                            .map(|c| Citation {
                                source: c.source_name,
                                snippet: c.excerpt,
                                relevance_score: c.relevance_score,
                            })
                            .collect();
                        proto_ConversationMessage {
                            role,
                            content: msg.content,
                            timestamp_unix_ms: msg.created_at.timestamp_millis(),
                            citations,
                        }
                    })
                    .collect();

                Ok(Response::new(GetSessionHistoryResponse {
                    messages: proto_messages,
                }))
            }
            Err(e) => Err(ai_error_to_status(e)),
        }
    }

    async fn clear_session(
        &self,
        request: Request<ClearSessionRequest>,
    ) -> Result<Response<ClearSessionResponse>, Status> {
        let req = request.into_inner();
        let session_id = parse_uuid(&req.session_id, "session_id")?;
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        // Verify session exists and belongs to operator before deleting
        match self.queries.get_session(session_id, operator_id).await {
            Ok(_session) => {
                // Delete session via repository
                match self.repo.delete(session_id).await {
                    Ok(()) => Ok(Response::new(ClearSessionResponse { cleared: true })),
                    Err(e) => Err(ai_error_to_status(e)),
                }
            }
            Err(e) => Err(ai_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

// Alias for the proto type to avoid naming conflict with domain type
type proto_ConversationMessage = platform_proto::ai_assistant::ConversationMessage;

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn ai_error_to_status(e: AiError) -> Status {
    match e {
        AiError::Unavailable(msg) => Status::unavailable(msg),
        AiError::InsufficientGrounding => {
            Status::failed_precondition("Insufficient data to answer the question")
        }
        AiError::QuotaExceeded => Status::resource_exhausted("AI usage quota exceeded"),
        AiError::QueryTooBroad { max_results } => {
            Status::invalid_argument(format!(
                "Query too broad: would return more than {} results",
                max_results
            ))
        }
        AiError::SessionNotFound(id) => {
            Status::not_found(format!("Conversation session not found: {}", id))
        }
        AiError::OperatorMismatch => Status::permission_denied("Operator ID mismatch"),
        AiError::RateLimitExceeded(qt) => {
            Status::unavailable(format!("Rate limit exceeded for query type: {}", qt))
        }
        AiError::MessageTooLong { length, max } => {
            Status::invalid_argument(format!(
                "Message too long: {} characters (max: {})",
                length, max
            ))
        }
    }
}

impl From<AiError> for Status {
    fn from(e: AiError) -> Self {
        ai_error_to_status(e)
    }
}
