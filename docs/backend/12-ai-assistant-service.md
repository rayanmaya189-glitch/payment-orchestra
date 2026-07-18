# 12 — ai-assistant-service (BC-12 AI Payment Assistant)

Read-only Conformist. RAG pipeline over tenant data. No write access to money-movement contexts.

---

## 1. Domain Model

### Read-Only Context (No Aggregate Roots)

**Internal State**:
- `ConversationSession` — bounded history window, tenant-scoped
- `GroundingCitation` — audit trail per answer
- OpenSearch index (RAG retrieval)

---

## 2. RAG Pipeline

### Ingestion Path

```
Domain Events → Chunking & Normalization → BGE-M3 embedding → OpenSearch index
Documents (OCR) → same pipeline
Prior Q&A (approved only) → same pipeline
```

### Query Path

1. Classify query: structured-lookup-eligible vs. retrieval-eligible
2. Embed query (BGE-M3)
3. Retrieve top-N from OpenSearch (dense + sparse hybrid)
4. Rerank to top-K (cross-encoder)
5. Assemble prompt (system instructions + retrieved chunks + conversation history)
6. Qwen3 32B generates answer
7. Validate citations exist in retrieved chunks
8. Return answer + citations

---

## 3. Guardrails

- **AI-P-001**: Grounding over fluency — cite sources or say "cannot answer"
- **AI-P-002**: Tenant isolation — separate OpenSearch index per tenant
- **AI-P-003**: No autonomous money movement — read-only
- **AI-EXFIL-001**: Output volume limits (100 records, 30 days max range)
- **AI-RATE-001**: Per-query-type rate limits
- **AIMON-006**: Circuit breaker → raw-data mode on quality drop

---

## 4. TDD Tests

```rust
#[tokio::test]
async fn test_answer_with_citations() {
    let result = handler.handle(AskAssistantQuery {
        question: "Why did our authorization rate drop yesterday?".into(),
        session_id: None,
    }).await.unwrap();
    assert!(!result.citations.is_empty());
    assert!(result.answer.contains("authorization rate"));
}

#[tokio::test]
async fn test_answer_declines_when_insufficient_data() {
    let result = handler.handle(AskAssistantQuery {
        question: "What will our revenue be next month?".into(),
        session_id: None,
    }).await.unwrap();
    assert!(result.answer.contains("cannot answer") || result.answer.contains("insufficient"));
}

#[tokio::test]
async fn test_cross_tenant_data_not_accessible() {
    // Query from tenant A should not return tenant B data
}

#[tokio::test]
async fn test_output_volume_limit_enforced() {
    // Query that would return >1000 records should prompt confirmation
}
```
