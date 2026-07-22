# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 6 of 12:** AI Payment Assistant & RAG Architecture
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 6 of 12 — AI Payment Assistant & RAG Architecture |
| Depends On | Part 3 (BC-12 read-only Conformist model), Part 4 (SVC-12 `ai-assistant-service`, SVC-18 `ai-gateway`) |
| Feeds Into | Part 8 (AI-specific security/compliance guardrails), Part 9 (vector index schema, OpenSearch design), Part 11 (GPU capacity planning, latency/throughput NFRs, evaluation-as-testing) |
| Model Stack | Ollama (self-hosted inference) running **Qwen3 32B** (reasoning/text), **Qwen3-VL 8B** (vision/document understanding), **BGE-M3** (multilingual embeddings) + a cross-encoder reranker |

---

## 1. Design Principles (Non-Negotiable, Restated from Part 1/2/3 for This Part's Context)

- **AI-P-001 (Grounding over fluency)**: Every substantive claim in an Assistant answer must be traceable to a specific retrieved record (transaction, event, document, or previously-approved report). If retrieval does not surface sufficient grounding, the Assistant says so explicitly rather than answering from the model's parametric knowledge (BIZ-023, EX-050a).
- **AI-P-002 (Tenant isolation is structural, not query-time filtering)**: Retrieval indices are physically partitioned per tenant (separate OpenSearch index per tenant, or a tenant-keyed shard routing scheme — Part 9 finalizes which), so that a retrieval bug cannot surface another tenant's data even transiently (BR-050-1).
- **AI-P-003 (No autonomous money movement)**: The Assistant has no command-side access to any bounded context (Part 3 §1.3). Every action it might "suggest" (e.g., resolving a reconciliation exception) must be executed by a human through the normal command API (BR-041-1).
- **AI-P-004 (Self-hosted by default)**: Model inference runs on platform-operator-controlled infrastructure via Ollama; no tenant data is sent to third-party model APIs unless a tenant explicitly opts in to a future "bring your own model provider" configuration (not in scope) — this satisfies BIZ-021 and the UAE data-residency assumption (ASSUMP-004, Part 1).
- **AI-P-005 (Not a regulated advice product)**: The Assistant answers operational/informational questions about the tenant's own data; it must not be positioned as, or allowed to produce, regulated financial/legal/tax advice (Part 1 §7.3 scope boundary note).

---

## 2. Model Version Management

### 2.1 Model Version Pinning

- **MODEL-PIN-001**: Every deployed model is pinned to a specific version with a cryptographic hash for integrity verification:
  - Qwen3 32B: pinned to specific model revision (e.g., `qwen3-32b-instruct-v1.0`)
  - Qwen3-VL 8B: pinned to specific model revision (e.g., `qwen3-vl-8b-instruct-v1.0`)
  - BGE-M3: pinned to specific model revision (e.g., `bge-m3-v1.0`)
  - Cross-encoder reranker: pinned to specific model revision

- **MODEL-PIN-002**: Model files are verified against a SHA-256 hash stored in a `model_versions` table (SeaORM entity):

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "model_versions")]
pub struct ModelVersionModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub model_id: String,           // e.g., "qwen3-32b"
    pub version: String,            // e.g., "v1.0"
    pub file_path: String,
    pub sha256_hash: String,
    pub deployed_at: DateTimeWithTimeZone,
    pub deployed_by: String,
    pub status: String,             // 'active' | 'deprecated' | 'rolled_back'
}
```

- **MODEL-PIN-003**: On Ollama startup, the `ai-assistant-service` verifies the loaded model's hash against the `model_versions` table. If the hash doesn't match, the service refuses to start and raises a critical alert (model integrity violation).

### 2.2 Model Rollback

- **MODEL-ROLLBACK-001**: Model updates are performed by:
  1. Deploying the new model version alongside the current version
  2. Running the evaluation suite (Part 6 §6) against the new version
  3. If evaluation passes, switching traffic to the new version
  4. If evaluation fails or quality degrades, rolling back to the previous version

- **MODEL-ROLLBACK-002**: Rollback is performed by updating the `model_versions` table to mark the current version as `rolled_back` and the previous version as `active`, then restarting the Ollama inference pool. Rollback takes effect within the restart window (typically < 30 seconds for model loading).

- **MODEL-ROLLBACK-003**: All model version changes (deployments, rollbacks) are logged in `change_history` (Part 3 §9.2 MKCK-004) with before/after version information.

## 3. Model Roles & Routing

| Model | Role | Invoked For | Typical Input |
|---|---|---|---|
| **Qwen3 32B** | Primary reasoning / answer generation | Natural-language Q&A (UC-050), anomaly explanation drafting, reconciliation-exception match suggestions, report narrative drafting | Assembled RAG prompt (retrieved context + conversation history + question) |
| **Qwen3-VL 8B** | Vision / document understanding | OCR and structured extraction from uploaded documents (bank settlement advices, trade licenses at KYB time, merchant-uploaded reconciliation files) | Image/PDF page renders + extraction instruction |
| **BGE-M3** | Embedding (dense + sparse + multi-vector retrieval support) | Indexing every ingested record/document; embedding every incoming query | Text chunks (Arabic and English both supported — BGE-M3's multilingual capability is specifically relevant given UAE bilingual business documentation, MKT context Part 1 §5) |
| **Cross-encoder reranker** | Precision re-ranking of initial retrieval candidates | Re-scoring the top-N (e.g., top 50) BGE-M3 retrieval hits down to the top-K (e.g., top 8) actually passed into the Qwen3 32B prompt | (query, candidate_chunk) pairs |

### 2.1 AI Gateway Routing Logic (extends Part 4 §3)

- **ROUTE-001**: Any request containing an uploaded image/PDF is routed first through Qwen3-VL 8B for extraction; the extracted structured text is then treated as additional retrievable/promptable context for a subsequent Qwen3 32B reasoning pass if the user's question requires reasoning over it (e.g., "does this settlement advice match invoice #4021?").
- **ROUTE-002**: Pure text operational questions (the majority of UC-050 traffic) go directly to the BGE-M3 → reranker → Qwen3 32B pipeline (§3).
- **ROUTE-003**: Model pools are provisioned on separate GPU node groups sized independently (Part 4 §8, Part 11) since Qwen3-VL 8B vision workloads and Qwen3 32B reasoning workloads have different memory/throughput profiles.

---

## 3. RAG Pipeline — End-to-End

### 3.1 Ingestion (Indexing Path — Always Running, Asynchronous)

```
Domain events (EVT-01…EVT-22, Part 3 §4)  ──┐
Documents (BC-13 DocumentRecord + OCR text) ─┼──► Chunking & Normalization ──► BGE-M3 embedding
Reconciliation exception records ───────────┘                                        │
Prior Assistant Q&A (approved/high-confidence only) ─────────────────────────────────┤
                                                                                       ▼
                                                                     Tenant-partitioned OpenSearch index
                                                                     (dense vectors + BM25 sparse fields)
```

- **ING-001**: Not every domain event is independently chunked/embedded as free text — high-volume, highly structured events (e.g., every single `PaymentAuthorizationAttempted`) are aggregated into periodically-refreshed **summary documents** (e.g., "hourly decline-reason summary per acquirer/scheme") *in addition to* remaining queryable in raw form via structured lookup (not vector search) for precise questions ("what happened to transaction X"). This dual-path design avoids forcing every precise, structured lookup through lossy semantic search, while still giving the Assistant summarized material for open-ended analytical questions.
- **ING-002**: Structured precise lookups (e.g., "status of payment_intent_id X") are served by direct, deterministic queries against the owning service's read model (Part 4 §4.1, direct read-model queries) — not by RAG retrieval at all. RAG is reserved for questions that genuinely require semantic retrieval over unstructured or loosely-structured content (documents, summaries, prior explanations), which keeps the Assistant's factual-lookup answers as accurate as the source-of-truth database rather than only as accurate as a vector search.
- **ING-003**: Re-embedding/index refresh runs as JOB-005 (Part 4 §6) on a schedule appropriate to data freshness needs (e.g., summary documents refreshed hourly; document-derived chunks embedded promptly on upload/OCR completion).

### 3.2 Query Path (Synchronous, User-Facing)

1. User submits a natural-language question (UC-050) via the dashboard or API.
2. `ai-gateway` (SVC-18) applies input guardrails (§5.1), resolves tenant context, checks AI-usage quota.
3. `ai-assistant-service` (SVC-12) classifies the query: **structured-lookup-eligible** (routes to ING-002's direct query path) vs. **retrieval-eligible** (proceeds to RAG).
4. For retrieval-eligible queries: embed query (BGE-M3) → retrieve top-N candidates from the tenant's partitioned OpenSearch index (dense + sparse hybrid search) → rerank to top-K (cross-encoder) → assemble prompt with: system instructions (including the grounding/citation requirement, AI-P-001), conversation history (bounded window), and the top-K retrieved chunks with source identifiers.
5. Qwen3 32B generates an answer. The prompt explicitly instructs the model to attach a source identifier to every factual claim; `ai-assistant-service` post-processes the raw output to validate that cited source identifiers actually correspond to retrieved chunks (rejecting/regenerating if the model fabricates a citation to a non-existent source — a deterministic guardrail, not solely a prompting request).
6. Answer + validated citations returned to the user; the full (query, retrieved context, answer, citations) tuple is logged to the `ai-gateway` guardrail audit log (Part 4 §3.2 AIGW-004) and to `ai-assistant-service`'s own `ConversationSession`/`GroundingCitation` records (Part 3 §5.8) for BIZ-023 compliance.

### 3.3 Conversation Session Management

- **SESS-001**: `ConversationSession` state (bounded history window, tenant-scoped) allows follow-up questions ("what about last week?") without requiring the user to restate context, while the bounded window prevents unbounded prompt growth from degrading latency/cost over a long session.
- **SESS-002**: Sessions are tenant- and user-scoped; no session ever mixes context across tenants or across users within a tenant (even though users within one tenant share the same underlying data, session history itself — what *this* user asked — is not shared with other users by default).

---

## 4. Document/Vision Pipeline (Qwen3-VL 8B)

### 4.1 Use Cases Served

- KYB evidence extraction (UC-002): trade license number, expiry, signatory name pre-fill and cross-check against manually entered fields.
- Settlement advice ingestion assist (PROC-05): when an acquirer only provides a scanned/PDF settlement advice rather than a structured file/API feed, Qwen3-VL 8B extracts line items into the `SettlementRecord` normalization pipeline (Part 3 BC-09, Part 7 connector capability flag for "unstructured settlement format").
- Ad hoc merchant document questions via the Assistant (AF-050a).

### 4.2 Processing Flow

1. Document uploaded → `document-service` (SVC-13) stores blob in MinIO, creates `DocumentRecord` (`Uploaded` status).
2. `document-service` triggers `ai-gateway` → Qwen3-VL 8B extraction job (async).
3. Extracted structured fields + confidence indicators returned; `DocumentRecord` transitions to `OcrCompleted` (or `OcrFailed` with reason).
4. Calling context (`compliance-service`, `reconciliation-service`, or `ai-assistant-service` for ad hoc Assistant use) consumes the structured extraction.

### 4.3 Guardrail Specific to Vision Extraction

- **VIS-001**: Extracted fields used for compliance-relevant decisions (e.g., KYB trade license expiry) are always presented to the human reviewer (ACT-06) alongside the original document image/PDF and a confidence indicator — the extraction pre-fills and flags discrepancies (AF-002a, Part 2) but never silently overrides a human-entered field or auto-approves a compliance case on extraction confidence alone.

---

## 5. Guardrails (Detailed)

### 5.1 Input Guardrails (AI Gateway, extends Part 4 §3.2 AIGW-003)

- **GRD-IN-001**: Basic prompt-injection pattern screening on any *externally-sourced* content that will enter a prompt (e.g., text extracted from a merchant-uploaded document, or free-text fields from an external acquirer's decline-reason description) — since these are less trusted than the platform's own structured domain events, they are treated as untrusted input requiring screening before being interpolated into a system-level prompt context.
- **GRD-IN-002**: Per-tenant/per-user rate limiting and quota enforcement (commercial + abuse-prevention).

### 5.2 Output Guardrails

- **GRD-OUT-001 (Citation validation)**: As described in §3.2 step 5 — fabricated citations are programmatically detected and trigger regeneration or an explicit "cannot ground this answer" fallback, not silent delivery of an uncited claim.
- **GRD-OUT-002 (No regulated-advice framing)**: System prompt and a post-generation classifier check jointly guard against the Assistant phrasing operational information as personalized financial/legal/tax advice (AI-P-005); borderline outputs are reframed to informational language ("your authorization rate for X was Y" rather than "you should switch acquirers because...") — the Assistant can surface data-driven observations but final business decisions are left explicitly to the human user.
- **GRD-OUT-003 (No autonomous action language)**: The Assistant's output must never imply it has already taken an action (e.g., "I've resolved the reconciliation exception") when it has only suggested one pending human confirmation (AI-P-003, BR-041-1) — this is enforced both by system prompt design and by the fact that, structurally, the Assistant has no command API to actually take such actions (Part 3 §1.3), so any such phrasing would be a hallucination the citation/consistency guardrail should also catch.

### 5.3 Escalation on Guardrail Failure

- **GRD-ESC-001**: Repeated guardrail failures for a given tenant/session (e.g., repeated prompt-injection attempts) are logged and surfaced to `ai-gateway`'s operational alert channel (ACT-07) rather than silently degrading — this is both a security signal and a product-quality signal.

---

## 6. Evaluation Harness

### 6.1 The "Top 50" Operational Questions (GOAL-004, SUCC-003)

Before GA, Product (STK-007) and Finance-Ops persona representatives (Part 1 §9, "Fatima") finalize a list of the 50 most common operational questions the Assistant must answer reliably, spanning categories such as:

- Reconciliation status ("what's still unmatched from yesterday's settlement batch?")
- Decline-reason analysis ("why did our authorization rate drop for Mastercard yesterday?")
- Settlement timing ("when will yesterday's captures settle?")
- Chargeback status ("what chargebacks are open and what's their deadline?")
- Fee/cost analysis ("how much did we pay in acquirer fees last month, broken down by acquirer?")

Each question in the final list is paired with a **ground-truth answer** (validated by a human against the actual underlying data) forming a regression test suite (Part 11 treats this as a first-class part of the TDD/testing strategy, not a one-off pre-launch check — the suite is re-run on every model/prompt/retrieval-pipeline change).

### 6.2 Evaluation Dimensions

| Dimension | What It Measures | Method |
|---|---|---|
| **Factual accuracy** | Does the answer's substantive content match ground truth? | Human-graded rubric + automated numeric-value extraction/comparison where the answer contains figures |
| **Citation validity** | Does every cited source actually support the claim it's attached to? | Automated check (GRD-OUT-001) + periodic human audit sample |
| **Grounding honesty** | Does the Assistant correctly decline to answer when it lacks sufficient grounding, rather than guessing? | Adversarial test set of intentionally unanswerable questions (EX-050a) |
| **Security** | Does the Assistant ever surface unauthorized data? | Automated security test suite against authorization boundaries (Part 8 security testing) |
| **Latency** | Time to first token / time to complete answer | Load-test harness (Part 11) |

### 6.3 Regression Gate

- **EVAL-001**: No change to the retrieval pipeline, prompt templates, or model version may be deployed to production if it regresses factual accuracy or citation validity below a threshold to be set by Product/QA in Part 11 — this evaluation suite is a release gate, not a dashboard nobody looks at.

---

## 7. Proactive Anomaly Detection (Phase 3, GOAL-010)

- **PROACT-001**: Building on the same summary-document ingestion path (§3.1 ING-001), an Phase 3 capability continuously compares current-period summary statistics (e.g., trailing-1-hour authorization rate per acquirer/scheme) against a learned/historical baseline and generates a candidate alert narrative when a statistically meaningful deviation is detected.
- **PROACT-002**: Candidate alerts are still subject to the same grounding/citation guardrails (§5.2) before being surfaced to a user via `notification-service` — an anomaly alert is itself an Assistant-generated artifact and must cite the specific underlying data driving the alert, not just assert "something looks off."
- This is explicitly **out of scope** (Part 1 §7.1 SCOPE-016) and recorded here only so Part 9's summary-document schema is designed with this future consumer in mind (avoiding a schema that would need to be redesigned to support it later).

---

## 8. Non-Functional Requirements Preview (Full Detail in Part 11)

- **NFR-AI-001**: Target time-to-first-token and time-to-complete-answer for a typical UC-050 question, to be benchmarked against actual Qwen3 32B throughput on the provisioned GPU node pool (Part 4 §8) — placeholder targets pending Part 11's capacity planning exercise.
- **NFR-AI-002**: `ai-assistant-service` availability target must account for graceful degradation (Part 4 §3.2 AIGW-005) — a degraded/unavailable Assistant must never block core payment orchestration (NFR-ORC-002, Part 5) or reconciliation workflows, which must remain fully usable via their normal dashboards/APIs without the Assistant.
- **NFR-AI-003**: GPU capacity sizing must account for concurrent load across both Qwen3 32B (reasoning) and Qwen3-VL 8B (vision) pools independently, since KYB-heavy onboarding periods and reconciliation-heavy month-end periods may create different peak-load shapes on each pool.

---

## 9. AI-Specific Gap Fixes

### 9.1 Production Model Quality Monitoring

- **AIMON-001**: A lightweight feedback loop is integrated into the Assistant UI: every answer includes a thumbs-up/thumbs-down feedback button. Feedback is stored per `(query, answer, session_id)` with timestamp, and aggregated daily into a quality-score dashboard accessible to the AI/ML team (STK-009).
- **AIMON-002**: A daily automated drift-detection job compares the current model's answer quality against the ground-truth "top 50" regression suite (§6.1). If factual accuracy drops below the EVAL-001 threshold, an alert is raised before any production degradation impacts merchants.
- **AIMON-003**: Retrieval quality metrics (average relevance score of top-K results, citation hit rate) are logged per query and aggregated into hourly rollups in ClickHouse, enabling trend analysis of retrieval pipeline health.

### 9.2 Prompt A/B Testing Framework

- **AIPROMPT-001**: A/B testing of system prompts and retrieval parameters is supported via a `prompt_variant` field on the `ConversationSession` aggregate. A configurable percentage of traffic is routed to the variant; quality metrics are tracked separately in ClickHouse.
- **AIPROMPT-002**: No prompt variant is promoted to 100% traffic unless it demonstrates statistically significant improvement over the baseline on the evaluation suite (§6.3 EVAL-001 gate applies to variants).

### 9.3 Conversation History Persistence and Export

- **AISESS-001**: Conversation sessions are persisted indefinitely (beyond the bounded prompt window, §3.3 SESS-001) in a `conversation_history` table, enabling search across past Q&A pairs, export/transcript capability for compliance review, and audit trail of AI interactions.

```sql
CREATE TABLE conversation_history (
    session_id      UUID NOT NULL,      -- UUIDv7
    message_seq     INT NOT NULL,
    role            TEXT NOT NULL,       -- 'user' | 'assistant'
    content         TEXT NOT NULL,
    citations       JSONB NULL,
    feedback        TEXT NULL,           -- 'positive' | 'negative' | NULL
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, message_seq)
);
```

- **AISESS-002**: The bounded prompt window (SESS-001) is a *prompt-construction* concern only — the full history is always stored, but only the most recent N messages are included in the LLM prompt.

### 9.4 Tool Use / Function Calling (Phase 2 Enhancement)

- **AITOOL-001**: The Assistant is extended with tool-use capability so it can invoke read-only API endpoints of other services to answer questions requiring fresh data:
  - `GetReconciliationExceptions(date_range)`
  - `GetPaymentIntentStatus(payment_intent_id)`
  - `GetAuthorizationRateStats(acquirer, scheme, period)`

- **AITOOL-002**: Tool calls are bounded to read-only endpoints — the Assistant has no write-path tool access, preserving AI-P-003 (no autonomous money movement).
- **AITOOL-003**: Tool call results are included in the RAG context and cited like any other retrieved source.
- **AITOOL-004**: Tool calls are gated by the same ABAC rules as direct API calls.

### 9.5 Multi-Step Reasoning Chains (Phase 2 Enhancement)

- **AICHAIN-001**: For complex questions requiring multiple retrieval rounds, the Assistant supports a multi-step reasoning chain: initial retrieval → self-evaluation → refined retrieval → final answer assembly.
- **AICHAIN-002**: The maximum number of reasoning steps is bounded (default: 3) to prevent unbounded latency growth. Each step's latency is tracked for NFR-AI-001 budget compliance.

### 9.6 Enhanced Prompt Injection Mitigation

- **GRD-IN-003**: The prompt-injection screening is formalized as a multi-layer defense:
  1. **Pattern blocklist**: Known injection patterns are blocked before entering the prompt context.
  2. **Content sandboxing**: Externally-sourced text is wrapped in `<external_content>` XML tags with instructions to treat as data, not instructions.
  3. **Output monitoring**: Post-generation classifier checks for leaked system prompt content.
  4. **Escalation**: Repeated injection attempts (≥3 per session or ≥5 per tenant per hour) are logged and the tenant's AI usage may be temporarily suspended.

---

## 10. Traceability

| Requirement | Realized By |
|---|---|
| BIZ-020 (grounded AI Assistant) | §3 RAG pipeline |
| BIZ-021 (self-hosted, data residency) | §1 AI-P-004, Ollama-hosted stack throughout |
| BIZ-022 (document/vision processing) | §4 |
| BIZ-023 (citable answers) | §3.2 step 5, §5.2 GRD-OUT-001 |
| BR-041-1 / BR-050-1 (human-in-the-loop, tenant isolation) | §1 AI-P-002/AI-P-003, §5.2 GRD-OUT-003 |
| GOAL-004 (top-50 baseline) | §6.1 |
| GOAL-010 / SCOPE-016 (proactive anomaly, Phase 3) | §7 |
| Part 1 §7.3 scope boundary (not financial advice) | §1 AI-P-005, §5.2 GRD-OUT-002 |
| Production model quality monitoring | §9.1 AIMON-001 through AIMON-003 |
| Prompt A/B testing | §9.2 AIPROMPT-001, AIPROMPT-002 |
| Conversation history persistence/export | §9.3 AISESS-001, AISESS-002 |
| Tool use / function calling (Phase 2) | §9.4 AITOOL-001 through AITOOL-004 |
| Multi-step reasoning chains (Phase 2) | §9.5 AICHAIN-001, AICHAIN-002 |
| Enhanced prompt injection mitigation | §9.6 GRD-IN-003 (layers 1–4) |

---

## 11. Gap Analysis Additions — AI Safety & Quality

### 11.1 AI Bias Detection & Fairness Monitoring

**AI-BIAS-003**: The evaluation harness (§6) is extended with a bias test set covering:
- Merchant size segments (small, medium, enterprise)
- Geographic segments (UAE, GCC, international)
- Transaction amount ranges (micro, standard, large-ticket)
- Card scheme segments (Visa, Mastercard, Amex, mada)

**AI-BIAS-004**: Fairness metric: compute answer accuracy per segment. If accuracy for any segment drops below 80% of the overall average, an alert is raised. This catches retrieval-pattern bias (e.g., over-indexing on high-volume merchants' patterns).

**AI-BIAS-005**: Quarterly bias audit: a random sample of 100 AI Assistant answers is reviewed by humans for fairness and consistency across merchant segments. Results are documented and available for compliance review.

### 11.2 Enhanced Hallucination Detection

**AI-HALL-002**: Secondary validation layer beyond citation existence (GRD-OUT-001):
- **Numerical claim extraction**: Parse the answer for numerical claims (amounts, percentages, counts) and cross-check against source documents. If a claimed number doesn't appear in any cited source, flag as "unverified numerical claim."
- **Source relevance scoring**: Use the cross-encoder reranker score as a "confidence" signal. Answers relying on low-relevance sources (reranker score < 0.5) trigger a disclaimer: "This answer may not be fully grounded in your data."
- **Contradiction detection**: If two cited sources contain contradictory information, the Assistant must explicitly note the contradiction rather than silently picking one.

**AI-HALL-003**: Periodic human audit of a random sample of answers (10% weekly sample, not just thumbs-up/down feedback). Audit checks: citation correctness (does the cited source actually support the claim?), numerical accuracy, and completeness.

### 11.3 Real-Time Production Quality Monitoring

**AIMON-004**: Hourly sampling of answer quality: automated checks on a rotating subset of the top-50 regression questions. If factual accuracy drops below the EVAL-001 threshold within any 1-hour window, an alert is raised (faster than the daily drift detection in AIMON-002).

**AIMON-005**: Real-time latency monitoring: p99 time-to-first-token and time-to-complete-answer tracked per model pool (Qwen3 32B, Qwen3-VL 8B). Alert if p99 exceeds 2x baseline.

**AIMON-006**: AI circuit breaker: if quality drops below threshold or latency exceeds 3x baseline within any 1-hour window, the AI Gateway degrades the Assistant to raw-data mode (AIGW-005 extended):
- **Q&A**: Returns unsummarized structured data with a "AI Assistant temporarily unavailable" notice
- **KYB OCR**: Routes to human review queue (ACT-06) with the uploaded document visible
- **Settlement OCR**: Queues for manual processing, alerts ACT-07

### 11.4 RAG Retrieval Quality Drift Detection

**AIMON-007**: Retrieval-specific quality metrics logged per query:
- `top_k_relevance_scores`: average similarity of top-K results from the cross-encoder reranker
- `citation_hit_rate`: percentage of cited sources that appear in the top-K retrieval results
- `no_results_rate`: percentage of queries returning zero retrieval results

**AIMON-008**: Hourly rollups compared against 7-day rolling average baseline. Alert if `citation_hit_rate` drops below 80% of baseline or `no_results_rate` exceeds 2x baseline.

**AIMON-009**: On drift detection, trigger JOB-005 (re-embedding) to refresh the index. If drift persists after re-embedding, escalate to AI/ML team for investigation.

### 11.5 Fraud Model Feedback Loop (Phase 1 Data Collection)

**FRAUD-FB-001**: A `FraudModelFeedback` event is emitted when: (a) a chargeback is received (linking back to the original `RiskAssessment`), or (b) a flagged transaction is confirmed legitimate by the merchant.

**FRAUD-FB-002**: A `fraud_feedback` ClickHouse table stores: `transaction_id`, `original_risk_score`, `original_risk_factors`, `outcome` (chargeback/legitimate/disputed), `outcome_date`, `feedback_lag_days`.

**FRAUD-FB-003**: A weekly analytics job computes model precision/recall/F1 from the feedback data. For Phase 1 rule-based models, these metrics are surfaced in the fraud analytics dashboard. For Phase 3 ML models, this table serves as the training data source.

---

## 12. Open Items Carried Forward

- **OQ-013**: Finalize the exact top-50 question list (§6.1) with Product/Finance-Ops persona input.
- **OQ-014**: Confirm GPU hardware specification/quantity (ties to Part 1 DEP-003 and Part 4 §8) before Part 11 finalizes NFR-AI-001 numeric latency targets.
- **OQ-015**: Decide the bounded conversation-history window size (§3.3 SESS-001).
- **OQ-039**: Finalize the feedback-loop UX design (§9.1 AIMON-001) — simple thumbs-up/down vs. structured feedback categories — affects quality dashboard granularity.
- **OQ-040**: Confirm tool-use API surface for Phase 2 (§9.4 AITOOL-001) — which read-only endpoints to expose, and whether tool results should be cached.
- **OQ-041**: Finalize multi-step reasoning step limit (§9.5 AICHAIN-002, default 3) based on latency benchmarks.
- **OQ-042**: Evaluate the trade-off between per-tenant OpenSearch indices (Part 9 OS-001) and a shared index with strong tenant-scoped query filtering — per-tenant provides stronger isolation but creates operational overhead at scale.

---

*End of Part 6. Proceed to Part 7: Gateway Connector Framework.*
