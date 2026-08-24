-- =============================================================================
-- Payment Orchestra — Core Platform Migration
-- =============================================================================
-- Applies to: payment_intents, routing_policies, payment_method_tokens,
--             gateway_profiles, principals, api_keys

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ─── Payment Intents ────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS payment_intents (
    payment_intent_id   UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id         UUID NOT NULL,
    amount_minor_units  BIGINT NOT NULL CHECK (amount_minor_units > 0),
    currency            VARCHAR(3) NOT NULL CHECK (length(currency) = 3),
    status              VARCHAR(32) NOT NULL DEFAULT 'created',
    payment_method_type VARCHAR(32) NOT NULL DEFAULT 'card',
    captured_amount_minor BIGINT NOT NULL DEFAULT 0,
    refunded_amount_minor BIGINT NOT NULL DEFAULT 0,
    routing_attempts    JSONB NOT NULL DEFAULT '[]',
    metadata_json       TEXT,
    error_message       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payment_intents_operator ON payment_intents(operator_id);
CREATE INDEX idx_payment_intents_status ON payment_intents(status);
CREATE INDEX idx_payment_intents_created ON payment_intents(created_at DESC);
CREATE INDEX idx_payment_intents_currency ON payment_intents(currency);

-- ─── Routing Policies ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS routing_policies (
    routing_policy_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id       UUID NOT NULL,
    name              VARCHAR(255) NOT NULL,
    status            VARCHAR(32) NOT NULL DEFAULT 'draft',
    rules             JSONB NOT NULL DEFAULT '[]',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    activated_at      TIMESTAMPTZ
);

CREATE INDEX idx_routing_policies_operator ON routing_policies(operator_id);
CREATE INDEX idx_routing_policies_status ON routing_policies(status);

-- ─── Payment Method Tokens ──────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS payment_method_tokens (
    token_id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id          UUID NOT NULL,
    token_status         VARCHAR(32) NOT NULL DEFAULT 'active',
    payment_method_type  VARCHAR(32) NOT NULL,
    token_ref            TEXT NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at           TIMESTAMPTZ
);

CREATE INDEX idx_pmt_operator ON payment_method_tokens(operator_id);
CREATE INDEX idx_pmt_status ON payment_method_tokens(token_status);

-- ─── Gateway Profiles ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS gateway_profiles (
    profile_id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id                 UUID NOT NULL,
    connector_id                VARCHAR(64) NOT NULL,
    merchant_acquirer_link_id   UUID NOT NULL,
    status                      VARCHAR(32) NOT NULL DEFAULT 'active',
    limits_min_amount_minor     BIGINT NOT NULL DEFAULT 0,
    limits_max_amount_minor     BIGINT NOT NULL DEFAULT 999999999,
    limits_daily_volume_minor   BIGINT NOT NULL DEFAULT 999999999999,
    limits_monthly_volume_minor BIGINT NOT NULL DEFAULT 999999999999999,
    limits_max_refund_minor     BIGINT NOT NULL DEFAULT 999999999,
    fee_fixed_minor             BIGINT NOT NULL DEFAULT 0,
    fee_percentage_bps          INTEGER NOT NULL DEFAULT 0,
    fee_cross_border_bps        INTEGER NOT NULL DEFAULT 0,
    fee_currency_conversion_bps INTEGER NOT NULL DEFAULT 0,
    fee_max_cap                 BIGINT,
    fee_min_floor               BIGINT,
    routing_priority            INTEGER NOT NULL DEFAULT 0,
    enabled_card_schemes        JSONB NOT NULL DEFAULT '["visa","mastercard"]',
    enabled_currencies          JSONB NOT NULL DEFAULT '["USD"]',
    enabled_countries           JSONB NOT NULL DEFAULT '["US"]',
    rate_limit_per_second       INTEGER NOT NULL DEFAULT 100,
    rate_limit_per_day          INTEGER NOT NULL DEFAULT 100000,
    rate_limit_burst            INTEGER NOT NULL DEFAULT 200,
    monitoring_success_rate_alert    DOUBLE PRECISION NOT NULL DEFAULT 0.95,
    monitoring_success_rate_critical DOUBLE PRECISION NOT NULL DEFAULT 0.90,
    monitoring_latency_p99_alert_ms    INTEGER NOT NULL DEFAULT 1000,
    monitoring_latency_p99_critical_ms INTEGER NOT NULL DEFAULT 3000,
    monitoring_auto_disable       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gp_operator ON gateway_profiles(operator_id);
CREATE INDEX idx_gp_connector ON gateway_profiles(connector_id);
CREATE INDEX idx_gp_status ON gateway_profiles(status);

-- ─── Principals (Users / Service Accounts) ──────────────────────────────────

CREATE TABLE IF NOT EXISTS principals (
    id                    UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    principal_type        VARCHAR(32) NOT NULL DEFAULT 'human',
    email                 VARCHAR(255) UNIQUE,
    password_hash         BYTEA,
    mfa_enrolled          BOOLEAN NOT NULL DEFAULT FALSE,
    mfa_method            VARCHAR(16),
    status                VARCHAR(32) NOT NULL DEFAULT 'active',
    roles                 TEXT NOT NULL DEFAULT '[]',
    permissions           TEXT NOT NULL DEFAULT '[]',
    operator_id           UUID,
    failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until          TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at         TIMESTAMPTZ,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_principals_email ON principals(email);
CREATE INDEX idx_principals_operator ON principals(operator_id);
CREATE INDEX idx_principals_status ON principals(status);

-- ─── API Keys ───────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS api_keys (
    api_key_id    UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    principal_id  UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    name          VARCHAR(255) NOT NULL,
    key_hash      BYTEA NOT NULL,
    scopes        JSONB NOT NULL DEFAULT '[]',
    status        VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ,
    last_used_at  TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_principal ON api_keys(principal_id);
CREATE INDEX idx_api_keys_status ON api_keys(status);
