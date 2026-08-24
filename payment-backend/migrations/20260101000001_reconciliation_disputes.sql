-- =============================================================================
-- Payment Orchestra — Reconciliation, Disputes, Risk, Notifications Migration
-- =============================================================================

-- ─── Settlement Batches ─────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS settlement_batches (
    settlement_batch_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    batch_file_name     VARCHAR(255) NOT NULL,
    status              VARCHAR(32) NOT NULL DEFAULT 'ingested',
    ingested_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    total_transactions  INTEGER NOT NULL DEFAULT 0,
    total_amount_minor  BIGINT NOT NULL DEFAULT 0,
    currency            VARCHAR(3) NOT NULL,
    records             JSONB NOT NULL DEFAULT '[]',
    file_checksum       VARCHAR(128) NOT NULL,
    matched_at          TIMESTAMPTZ,
    quarantined_at      TIMESTAMPTZ
);

CREATE INDEX idx_sb_status ON settlement_batches(status);
CREATE INDEX idx_sb_currency ON settlement_batches(currency);
CREATE INDEX idx_sb_ingested ON settlement_batches(ingested_at DESC);

-- ─── Ledger Entries ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ledger_entries (
    entry_id        UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id  UUID NOT NULL,
    entry_type      VARCHAR(32) NOT NULL,
    amount_minor    BIGINT NOT NULL,
    currency        VARCHAR(3) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ledger_txn ON ledger_entries(transaction_id);
CREATE INDEX idx_ledger_type ON ledger_entries(entry_type);

-- ─── Settlement Expectations ────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS settlement_expectations (
    expectation_id    UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id    UUID NOT NULL,
    expected_amount   BIGINT NOT NULL,
    expected_currency VARCHAR(3) NOT NULL,
    settlement_date   DATE NOT NULL,
    status            VARCHAR(32) NOT NULL DEFAULT 'pending',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_se_txn ON settlement_expectations(transaction_id);
CREATE INDEX idx_se_status ON settlement_expectations(status);
CREATE INDEX idx_se_date ON settlement_expectations(settlement_date);

-- ─── Fee Variances ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS fee_variances (
    variance_id      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id   UUID NOT NULL,
    expected_fee     BIGINT NOT NULL,
    actual_fee       BIGINT NOT NULL,
    variance_amount  BIGINT NOT NULL,
    status           VARCHAR(32) NOT NULL DEFAULT 'detected',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fv_txn ON fee_variances(transaction_id);
CREATE INDEX idx_fv_status ON fee_variances(status);

-- ─── Chargeback Cases ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS chargeback_cases (
    chargeback_id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id            UUID NOT NULL,
    payment_intent_id      UUID NOT NULL,
    acquirer_link_id       UUID NOT NULL,
    status                 VARCHAR(32) NOT NULL DEFAULT 'open',
    reason_code            VARCHAR(64) NOT NULL,
    amount_minor_units     BIGINT NOT NULL,
    currency               VARCHAR(3) NOT NULL,
    received_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    representment_deadline TIMESTAMPTZ NOT NULL,
    resolved_at            TIMESTAMPTZ,
    outcome                VARCHAR(32),
    resolution_note        TEXT,
    submissions            JSONB NOT NULL DEFAULT '[]'
);

CREATE INDEX idx_cb_operator ON chargeback_cases(operator_id);
CREATE INDEX idx_cb_payment ON chargeback_cases(payment_intent_id);
CREATE INDEX idx_cb_status ON chargeback_cases(status);
CREATE INDEX idx_cb_deadline ON chargeback_cases(representment_deadline);

-- ─── Risk Assessments ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS risk_assessments (
    risk_assessment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_intent_id  UUID NOT NULL,
    risk_score         DOUBLE PRECISION NOT NULL CHECK (risk_score >= 0 AND risk_score <= 1),
    risk_level         VARCHAR(16) NOT NULL,
    risk_factors       JSONB NOT NULL DEFAULT '[]',
    rule_version       VARCHAR(64) NOT NULL,
    assessed_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ra_payment ON risk_assessments(payment_intent_id);
CREATE INDEX idx_ra_level ON risk_assessments(risk_level);

-- ─── Notifications ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS notifications (
    notification_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id     UUID NOT NULL,
    channel         VARCHAR(16) NOT NULL,
    recipient       VARCHAR(255) NOT NULL,
    template_id     VARCHAR(128) NOT NULL,
    payload_json    TEXT NOT NULL,
    subject         VARCHAR(255),
    status          VARCHAR(32) NOT NULL DEFAULT 'pending',
    retry_count     INTEGER NOT NULL DEFAULT 0,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at         TIMESTAMPTZ,
    last_error      TEXT
);

CREATE INDEX idx_notif_operator ON notifications(operator_id);
CREATE INDEX idx_notif_status ON notifications(status);
CREATE INDEX idx_notif_channel ON notifications(channel);
CREATE INDEX idx_notif_created ON notifications(created_at DESC);

-- ─── Analytics Events ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS analytics_events (
    event_id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type          VARCHAR(64) NOT NULL,
    payment_intent_id   UUID,
    operator_id         UUID,
    acquirer_id         VARCHAR(64),
    card_scheme         VARCHAR(32),
    currency            VARCHAR(3),
    amount_minor_units  BIGINT,
    decline_reason      VARCHAR(128),
    latency_ms          INTEGER,
    acquirer_fee        BIGINT,
    chargeback_amount   BIGINT,
    chargeback_reason   TEXT,
    fraud_score         DOUBLE PRECISION,
    bin                 VARCHAR(16),
    country_code        VARCHAR(2),
    merchant_id         VARCHAR(128),
    failover_routed     BOOLEAN,
    occurred_at         TIMESTAMPTZ NOT NULL,
    ingested_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ae_type ON analytics_events(event_type);
CREATE INDEX idx_ae_operator ON analytics_events(operator_id);
CREATE INDEX idx_ae_payment ON analytics_events(payment_intent_id);
CREATE INDEX idx_ae_occurred ON analytics_events(occurred_at DESC);

-- ─── Invoices ───────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS invoices (
    invoice_id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id         UUID NOT NULL,
    order_reference     VARCHAR(128) NOT NULL,
    status              VARCHAR(32) NOT NULL DEFAULT 'draft',
    line_items          JSONB NOT NULL DEFAULT '[]',
    total_amount_minor  BIGINT NOT NULL DEFAULT 0,
    paid_amount_minor   BIGINT NOT NULL DEFAULT 0,
    currency            VARCHAR(3) NOT NULL,
    due_date            TIMESTAMPTZ NOT NULL,
    recipient_email     VARCHAR(255),
    payment_intent_ids  JSONB NOT NULL DEFAULT '[]',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_inv_operator ON invoices(operator_id);
CREATE INDEX idx_inv_status ON invoices(status);
CREATE INDEX idx_inv_due ON invoices(due_date);
