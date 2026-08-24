-- =============================================================================
-- Payment Orchestra — SaaS Billing, Subscriptions, Supporting Services
-- =============================================================================

-- ─── SaaS Plans ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS saas_plans (
    plan_id                UUID PRIMARY KEY,
    name                   VARCHAR(128) NOT NULL,
    slug                   VARCHAR(128) UNIQUE NOT NULL,
    description            TEXT,
    price_monthly_minor    BIGINT NOT NULL DEFAULT 0,
    price_per_txn_minor    BIGINT NOT NULL DEFAULT 0,
    included_txns_monthly  INTEGER NOT NULL DEFAULT 0,
    max_gateways           INTEGER NOT NULL DEFAULT 1,
    max_team_members       INTEGER NOT NULL DEFAULT 1,
    max_api_keys           INTEGER NOT NULL DEFAULT 1,
    max_webhooks           INTEGER NOT NULL DEFAULT 1,
    data_retention_days    INTEGER NOT NULL DEFAULT 30,
    features               JSONB NOT NULL DEFAULT '{}',
    is_active              BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order             INTEGER NOT NULL DEFAULT 0,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── Tenant Subscriptions ───────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS tenant_subscriptions (
    subscription_id       UUID PRIMARY KEY,
    operator_id           UUID NOT NULL,
    plan_id               UUID NOT NULL REFERENCES saas_plans(plan_id),
    status                VARCHAR(32) NOT NULL DEFAULT 'trialing',
    current_period_start  TIMESTAMPTZ NOT NULL,
    current_period_end    TIMESTAMPTZ NOT NULL,
    trial_ends_at         TIMESTAMPTZ,
    canceled_at           TIMESTAMPTZ,
    cancel_reason         TEXT,
    payment_method_id     VARCHAR(128),
    stripe_subscription_id VARCHAR(128),
    created_by            UUID NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ts_operator ON tenant_subscriptions(operator_id);
CREATE INDEX idx_ts_plan ON tenant_subscriptions(plan_id);
CREATE INDEX idx_ts_status ON tenant_subscriptions(status);

-- ─── Tenant Usage ───────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS tenant_usage (
    usage_id                  UUID PRIMARY KEY,
    operator_id               UUID NOT NULL,
    period_start              TIMESTAMPTZ NOT NULL,
    period_end                TIMESTAMPTZ NOT NULL,
    transaction_count         INTEGER NOT NULL DEFAULT 0,
    transaction_volume_minor  BIGINT NOT NULL DEFAULT 0,
    api_calls                 INTEGER NOT NULL DEFAULT 0,
    storage_bytes             BIGINT NOT NULL DEFAULT 0,
    ai_queries                INTEGER NOT NULL DEFAULT 0,
    overage_amount_minor      BIGINT NOT NULL DEFAULT 0,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tu_operator ON tenant_usage(operator_id);
CREATE UNIQUE INDEX idx_tu_operator_period ON tenant_usage(operator_id, period_start);

-- ─── SaaS Invoices ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS saas_invoices (
    invoice_id          UUID PRIMARY KEY,
    operator_id         UUID NOT NULL,
    subscription_id     UUID NOT NULL REFERENCES tenant_subscriptions(subscription_id),
    invoice_number      VARCHAR(64) NOT NULL,
    status              VARCHAR(32) NOT NULL DEFAULT 'draft',
    subtotal_minor      BIGINT NOT NULL DEFAULT 0,
    tax_minor           BIGINT NOT NULL DEFAULT 0,
    total_minor         BIGINT NOT NULL DEFAULT 0,
    currency            VARCHAR(3) NOT NULL DEFAULT 'USD',
    period_start        TIMESTAMPTZ NOT NULL,
    period_end          TIMESTAMPTZ NOT NULL,
    due_date            TIMESTAMPTZ NOT NULL,
    paid_at             TIMESTAMPTZ,
    line_items          JSONB NOT NULL DEFAULT '[]',
    stripe_invoice_id   VARCHAR(128),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_si_operator ON saas_invoices(operator_id);
CREATE INDEX idx_si_subscription ON saas_invoices(subscription_id);
CREATE INDEX idx_si_status ON saas_invoices(status);

-- ─── Tenant Team Members ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS tenant_team_members (
    membership_id  UUID PRIMARY KEY,
    operator_id    UUID NOT NULL,
    principal_id   UUID NOT NULL,
    role           VARCHAR(32) NOT NULL DEFAULT 'member',
    invited_by     UUID NOT NULL,
    invited_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at    TIMESTAMPTZ,
    status         VARCHAR(32) NOT NULL DEFAULT 'pending',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ttm_operator ON tenant_team_members(operator_id);
CREATE INDEX idx_ttm_principal ON tenant_team_members(principal_id);

-- ─── Audit Logs ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS audit_logs (
    log_id        UUID PRIMARY KEY,
    operator_id   UUID NOT NULL,
    principal_id  UUID NOT NULL,
    action        VARCHAR(128) NOT NULL,
    resource      VARCHAR(128) NOT NULL,
    resource_id   VARCHAR(128),
    old_value     JSONB,
    new_value     JSONB,
    ip_address    INET,
    user_agent    TEXT,
    metadata      JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_al_operator ON audit_logs(operator_id);
CREATE INDEX idx_al_action ON audit_logs(action);
CREATE INDEX idx_al_created ON audit_logs(created_at DESC);

-- ─── Subscriptions ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS subscriptions (
    subscription_id  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id      UUID NOT NULL,
    plan_slug        VARCHAR(64) NOT NULL,
    status           VARCHAR(32) NOT NULL DEFAULT 'active',
    billing_cycle    VARCHAR(16) NOT NULL DEFAULT 'monthly',
    current_period_start TIMESTAMPTZ NOT NULL,
    current_period_end   TIMESTAMPTZ NOT NULL,
    cancel_at        TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sub_operator ON subscriptions(operator_id);
CREATE INDEX idx_sub_status ON subscriptions(status);

-- ─── Payment Links ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS payment_links (
    link_id        UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id    UUID NOT NULL,
    token          VARCHAR(64) UNIQUE NOT NULL,
    title          VARCHAR(255) NOT NULL,
    description    TEXT,
    amount_minor   BIGINT,
    currency       VARCHAR(3),
    status         VARCHAR(32) NOT NULL DEFAULT 'active',
    max_uses       INTEGER,
    use_count      INTEGER NOT NULL DEFAULT 0,
    expires_at     TIMESTAMPTZ,
    redirect_url   TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pl_operator ON payment_links(operator_id);
CREATE INDEX idx_pl_token ON payment_links(token);

-- ─── Onboarding Requests ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS onboarding_requests (
    request_id     UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id    UUID NOT NULL,
    connector_id   VARCHAR(64) NOT NULL,
    status         VARCHAR(32) NOT NULL DEFAULT 'pending',
    config_json    JSONB NOT NULL DEFAULT '{}',
    submitted_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at   TIMESTAMPTZ,
    error_message  TEXT
);

CREATE INDEX idx_or_operator ON onboarding_requests(operator_id);
CREATE INDEX idx_or_status ON onboarding_requests(status);

-- ─── Scheduled Jobs ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS scheduled_jobs (
    job_id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_type       VARCHAR(64) NOT NULL,
    payload_json   JSONB NOT NULL DEFAULT '{}',
    status         VARCHAR(32) NOT NULL DEFAULT 'pending',
    scheduled_at   TIMESTAMPTZ NOT NULL,
    started_at     TIMESTAMPTZ,
    completed_at   TIMESTAMPTZ,
    error_message  TEXT,
    retry_count    INTEGER NOT NULL DEFAULT 0,
    max_retries    INTEGER NOT NULL DEFAULT 3,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sj_status ON scheduled_jobs(status);
CREATE INDEX idx_sj_scheduled ON scheduled_jobs(scheduled_at);
CREATE INDEX idx_sj_type ON scheduled_jobs(job_type);

-- ─── Saga Executions ────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS saga_executions (
    saga_id        UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    saga_type      VARCHAR(64) NOT NULL,
    status         VARCHAR(32) NOT NULL DEFAULT 'running',
    payload_json   JSONB NOT NULL DEFAULT '{}',
    current_step   INTEGER NOT NULL DEFAULT 0,
    total_steps    INTEGER NOT NULL DEFAULT 0,
    started_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at   TIMESTAMPTZ,
    error_message  TEXT
);

CREATE INDEX idx_se_type ON saga_executions(saga_type);
CREATE INDEX idx_saga_exec_status ON saga_executions(status);

-- ─── KYB Cases ──────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS kyb_cases (
    case_id       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id   UUID NOT NULL,
    status        VARCHAR(32) NOT NULL DEFAULT 'pending',
    documents     JSONB NOT NULL DEFAULT '[]',
    submitted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at   TIMESTAMPTZ,
    reviewer_id   UUID,
    decision      VARCHAR(32),
    notes         TEXT
);

CREATE INDEX idx_kyb_operator ON kyb_cases(operator_id);
CREATE INDEX idx_kyb_status ON kyb_cases(status);

-- ─── AML Alerts ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS aml_alerts (
    alert_id      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id   UUID NOT NULL,
    alert_type    VARCHAR(64) NOT NULL,
    severity      VARCHAR(16) NOT NULL DEFAULT 'medium',
    description   TEXT NOT NULL,
    metadata      JSONB NOT NULL DEFAULT '{}',
    status        VARCHAR(32) NOT NULL DEFAULT 'open',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at   TIMESTAMPTZ,
    resolved_by   UUID
);

CREATE INDEX idx_aml_operator ON aml_alerts(operator_id);
CREATE INDEX idx_aml_status ON aml_alerts(status);
CREATE INDEX idx_aml_severity ON aml_alerts(severity);

-- ─── AI Gateway Queries ─────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ai_gateway_queries (
    query_id      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id   UUID NOT NULL,
    query_text    TEXT NOT NULL,
    response_text TEXT,
    model         VARCHAR(64) NOT NULL,
    tokens_used   INTEGER NOT NULL DEFAULT 0,
    latency_ms    INTEGER,
    status        VARCHAR(32) NOT NULL DEFAULT 'pending',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_aig_operator ON ai_gateway_queries(operator_id);
CREATE INDEX idx_aig_created ON ai_gateway_queries(created_at DESC);

-- ─── Conversation Sessions ──────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS conversation_sessions (
    session_id    UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id   UUID NOT NULL,
    title         VARCHAR(255),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cs_operator ON conversation_sessions(operator_id);

-- ─── Event Store ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS event_store (
    event_id      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id  UUID NOT NULL,
    aggregate_type VARCHAR(64) NOT NULL,
    event_type    VARCHAR(128) NOT NULL,
    event_data    JSONB NOT NULL,
    metadata      JSONB,
    version       INTEGER NOT NULL,
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_es_aggregate ON event_store(aggregate_id, version);
CREATE INDEX idx_es_type ON event_store(event_type);
CREATE INDEX idx_es_occurred ON event_store(occurred_at DESC);

-- ─── Aggregate Snapshots ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS aggregate_snapshots (
    snapshot_id    UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id   UUID NOT NULL,
    aggregate_type VARCHAR(64) NOT NULL,
    version        INTEGER NOT NULL,
    snapshot_data  JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_as_aggregate ON aggregate_snapshots(aggregate_id);

-- ─── Outbox Entries ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS outbox_entries (
    entry_id       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id   UUID NOT NULL,
    event_type     VARCHAR(128) NOT NULL,
    payload        JSONB NOT NULL,
    published      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at   TIMESTAMPTZ
);

CREATE INDEX idx_ob_unpublished ON outbox_entries(published, created_at);
