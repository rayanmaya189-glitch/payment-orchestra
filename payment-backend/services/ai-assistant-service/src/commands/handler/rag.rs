use async_trait::async_trait;
use uuid::Uuid;

use super::RagEngine;
use crate::domain::*;

pub fn classify_query(question: &str) -> QueryClassification {
    let lower = question.to_lowercase();

    let future_keywords = [
        "will happen", "next month", "next year", "future", "predict",
        "forecast", "projection", "stock", "market", "competitor",
    ];
    if future_keywords.iter().any(|k| lower.contains(k)) {
        return QueryClassification::Unanswerable;
    }

    let lookup_keywords = [
        "rate", "amount", "count", "how many", "total", "sum",
        "yesterday", "today", "last week", "last month", "trend",
        "decline", "authorization", "revenue", "fees", "chargeback",
        "settlement", "volume",
    ];
    if lookup_keywords.iter().any(|k| lower.contains(k)) {
        return QueryClassification::StructuredLookup;
    }

    QueryClassification::RetrievalEligible
}

pub struct SimulatedRagEngine;

#[async_trait]
impl RagEngine for SimulatedRagEngine {
    async fn answer_question(
        &self,
        question: &str,
        classification: QueryClassification,
        _context: &ConversationSession,
    ) -> Result<RagResult, AiError> {
        let lower = question.to_lowercase();
        let start = std::time::Instant::now();

        let (answer, citations, confidence) = if classification == QueryClassification::StructuredLookup {
            mock_lookup_response(&lower)
        } else {
            mock_retrieval_response(&lower)
        };

        let elapsed = start.elapsed().as_millis() as u64;

        Ok(RagResult {
            answer,
            citations,
            confidence,
            processing_time_ms: elapsed,
        })
    }
}

fn mock_lookup_response(question: &str) -> (String, Vec<GroundingCitation>, AnswerConfidence) {
    if question.contains("rate") || question.contains("authorization") {
        (
            "Based on the last 24 hours of transaction data, your authorization rate is 94.3%. \
             This is within the typical range of 90-97% for your payment profile. \
             Visa transactions are performing at 95.1%, Mastercard at 93.8%."
                .into(),
            vec![GroundingCitation {
                citation_id: Uuid::now_v7(),
                source_type: CitationSourceType::PaymentEvent,
                source_id: "analytics:auth_rate:last_24h".into(),
                source_name: "Authorization Rate Report".into(),
                excerpt: "Visa auth rate: 95.1%, Mastercard auth rate: 93.8%, Overall: 94.3%".into(),
                relevance_score: 0.95,
            }],
            AnswerConfidence::High,
        )
    } else if question.contains("decline") {
        (
            "The most common decline reasons in the last 7 days are:\n\
             1. Insufficient funds — 42.3%\n\
             2. Do not honor — 28.1%\n\
             3. Pickup card — 12.5%\n\
             4. Expired card — 10.2%\n\
             5. Other — 6.9%\n\n\
             Insufficient funds is above your usual rate of 35%. Consider checking \
             the average transaction amount vs. available balance patterns."
                .into(),
            vec![GroundingCitation {
                citation_id: Uuid::now_v7(),
                source_type: CitationSourceType::PaymentEvent,
                source_id: "analytics:decline_reasons:last_7d".into(),
                source_name: "Decline Reason Analysis".into(),
                excerpt: "Insufficient funds: 42.3%, Do not honor: 28.1%, Pickup card: 12.5%".into(),
                relevance_score: 0.92,
            }],
            AnswerConfidence::High,
        )
    } else if question.contains("chargeback") {
        (
            "Your chargeback rate over the last 30 days is 0.45%, which is well below \
             the Visa threshold of 1.5% and Mastercard threshold of 1.8%. \
             You have received 12 chargebacks out of 2,667 transactions. \
             The primary reason is fraud (58.3%), followed by service not received (25.0%)."
                .into(),
            vec![
                GroundingCitation {
                    citation_id: Uuid::now_v7(),
                    source_type: CitationSourceType::PaymentEvent,
                    source_id: "analytics:chargeback_trends:30d".into(),
                    source_name: "Chargeback Trend Report".into(),
                    excerpt: "Chargeback rate: 0.45%, Total chargebacks: 12, Total transactions: 2,667".into(),
                    relevance_score: 0.94,
                },
            ],
            AnswerConfidence::High,
        )
    } else if question.contains("fee") || question.contains("revenue") {
        (
            "Total processing fees for last month were $12,450 across 8,230 transactions. \
             The average fee per transaction is $1.51. \
             Network International has the lowest average fee at $1.32, \
             while Checkout.com averages $1.68 per transaction."
                .into(),
            vec![GroundingCitation {
                citation_id: Uuid::now_v7(),
                source_type: CitationSourceType::PaymentEvent,
                source_id: "analytics:fee_analysis:last_month".into(),
                source_name: "Fee Analysis Report".into(),
                excerpt: "Total fees: $12,450, Avg fee: $1.51, Transactions: 8,230".into(),
                relevance_score: 0.91,
            }],
            AnswerConfidence::High,
        )
    } else {
        (
            "I found the following information related to your query:\n\n\
                 Based on your transaction data for the current period, \
                 there are relevant records available. Here's a summary of what I found:\n\n\
                 - Total transactions: 12,450\n\
                 - Success rate: 94.2%\n\
                 - Average transaction value: $85.32\n\n\
                 Would you like me to drill down into any specific aspect?".to_string(),
            vec![GroundingCitation {
                citation_id: Uuid::now_v7(),
                source_type: CitationSourceType::KnowledgeBase,
                source_id: "analytics:overview".into(),
                source_name: "Transaction Overview".into(),
                excerpt: "Total transactions: 12,450, Success rate: 94.2%".into(),
                relevance_score: 0.85,
            }],
            AnswerConfidence::Medium,
        )
    }
}

fn mock_retrieval_response(question: &str) -> (String, Vec<GroundingCitation>, AnswerConfidence) {
    let lower = question.to_lowercase();

    if lower.contains("how") || lower.contains("setup") || lower.contains("configure") {
        (
            "Based on the documentation and knowledge base, here are the steps:\n\n\
             1. Navigate to Settings > Payment Connectors in your dashboard\n\
             2. Click 'Add Connector' and select your provider\n\
             3. Enter your API credentials (secret key, merchant ID, etc.)\n\
             4. Configure routing rules in the Orchestration section\n\
             5. Test the connection before going live\n\n\
             Make sure to use sandbox credentials for initial testing."
                .into(),
            vec![GroundingCitation {
                citation_id: Uuid::now_v7(),
                source_type: CitationSourceType::KnowledgeBase,
                source_id: "docs:connector_setup".into(),
                source_name: "Connector Setup Guide".into(),
                excerpt: "Step-by-step connector configuration including credential entry and testing".into(),
                relevance_score: 0.93,
            }],
            AnswerConfidence::High,
        )
    } else if lower.contains("refund") || lower.contains("void") {
        (
            "To process a refund:\n\n\
             1. Go to Payments > Payment Management\n\
             2. Find the transaction using the payment ID or customer email\n\
             3. Click the transaction to open details\n\
             4. Select 'Refund' from the action menu\n\
             5. Enter the refund amount (partial or full)\n\
             6. Add a reason for the refund\n\
             7. Confirm to submit\n\n\
             Note: Refunds can only be processed within 120 days of the original \
             transaction. Full refunds must be processed within the same currency."
                .into(),
            vec![GroundingCitation {
                citation_id: Uuid::now_v7(),
                source_type: CitationSourceType::KnowledgeBase,
                source_id: "docs:refund_processing".into(),
                source_name: "Refund Processing Guide".into(),
                excerpt: "Refund processing steps including eligibility and time limits".into(),
                relevance_score: 0.95,
            }],
            AnswerConfidence::High,
        )
    } else if lower.contains("reconciliation") {
        (
            "The reconciliation process matches your settlement reports against \
             your transaction records. Here's how to use it:\n\n\
             1. Upload your settlement CSV from your acquirer\n\
             2. The system automatically matches transactions\n\
             3. Review unmatched items in the Reconciliation tab\n\
             4. Manually resolve any discrepancies\n\
             5. Export the reconciliation report for your records\n\n\
             Your current reconciliation rate is 98.7%."
                .into(),
            vec![GroundingCitation {
                citation_id: Uuid::now_v7(),
                source_type: CitationSourceType::Document,
                source_id: "docs:reconciliation_guide".into(),
                source_name: "Reconciliation User Guide".into(),
                excerpt: "Settlement reconciliation process overview and best practices".into(),
                relevance_score: 0.90,
            }],
            AnswerConfidence::Medium,
        )
    } else {
        (
            "I understand you're asking about this topic. Based on the available \
             knowledge base and documentation, I don't have specific enough \
             information to provide a detailed answer. Could you please rephrase \
             your question or provide more context?"
                .into(),
            vec![],
            AnswerConfidence::Low,
        )
    }
}
