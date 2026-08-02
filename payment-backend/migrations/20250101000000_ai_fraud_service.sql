-- AI Fraud Service Tables
-- Version: 20250101000000

-- Fraud check results
CREATE TABLE IF NOT EXISTS fraud_checks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL,
    risk_score DOUBLE PRECISION NOT NULL CHECK (risk_score >= 0.0 AND risk_score <= 1.0),
    decision VARCHAR(20) NOT NULL CHECK (decision IN ('Approve', 'Review', 'Decline')),
    reasons JSONB NOT NULL DEFAULT '[]',
    model_version VARCHAR(50) NOT NULL,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fraud_checks_transaction_id ON fraud_checks(transaction_id);
CREATE INDEX idx_fraud_checks_decision ON fraud_checks(decision);
CREATE INDEX idx_fraud_checks_checked_at ON fraud_checks(checked_at);

-- Transaction records for velocity and pattern analysis
CREATE TABLE IF NOT EXISTS fraud_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL UNIQUE,
    merchant_id UUID NOT NULL,
    customer_key VARCHAR(255) NOT NULL, -- email or IP
    amount_minor BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL,
    card_bin VARCHAR(10),
    card_last_four VARCHAR(4),
    card_country VARCHAR(2),
    customer_ip INET,
    device_id VARCHAR(255),
    billing_country VARCHAR(2),
    shipping_country VARCHAR(2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fraud_transactions_customer_key ON fraud_transactions(customer_key);
CREATE INDEX idx_fraud_transactions_device_id ON fraud_transactions(device_id);
CREATE INDEX idx_fraud_transactions_created_at ON fraud_transactions(created_at);
CREATE INDEX idx_fraud_transactions_merchant_id ON fraud_transactions(merchant_id);

-- Device fingerprint tracking
CREATE TABLE IF NOT EXISTS fraud_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id VARCHAR(255) NOT NULL UNIQUE,
    customer_keys TEXT[] NOT NULL DEFAULT '{}',
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    risk_score DOUBLE PRECISION DEFAULT 0.0,
    is_blocked BOOLEAN DEFAULT FALSE,
    metadata JSONB DEFAULT '{}'
);

CREATE INDEX idx_fraud_devices_device_id ON fraud_devices(device_id);
CREATE INDEX idx_fraud_devices_risk_score ON fraud_devices(risk_score);

-- Email reputation tracking
CREATE TABLE IF NOT EXISTS fraud_email_reputation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    domain VARCHAR(255) NOT NULL,
    is_disposable BOOLEAN DEFAULT FALSE,
    is_known_fraud BOOLEAN DEFAULT FALSE,
    account_age_seconds BIGINT DEFAULT 0,
    transaction_count INTEGER DEFAULT 0,
    total_amount_minor BIGINT DEFAULT 0,
    risk_score DOUBLE PRECISION DEFAULT 0.0,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fraud_email_reputation_email ON fraud_email_reputation(email);
CREATE INDEX idx_fraud_email_reputation_domain ON fraud_email_reputation(domain);
CREATE INDEX idx_fraud_email_reputation_risk_score ON fraud_email_reputation(risk_score);

-- IP reputation tracking
CREATE TABLE IF NOT EXISTS fraud_ip_reputation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ip_address INET NOT NULL UNIQUE,
    country VARCHAR(2),
    is_vpn BOOLEAN DEFAULT FALSE,
    is_tor BOOLEAN DEFAULT FALSE,
    is_proxy BOOLEAN DEFAULT FALSE,
    transaction_count INTEGER DEFAULT 0,
    failed_count INTEGER DEFAULT 0,
    risk_score DOUBLE PRECISION DEFAULT 0.0,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fraud_ip_reputation_ip ON fraud_ip_reputation(ip_address);
CREATE INDEX idx_fraud_ip_reputation_risk_score ON fraud_ip_reputation(risk_score);

-- Fraud rules configuration
CREATE TABLE IF NOT EXISTS fraud_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_name VARCHAR(100) NOT NULL UNIQUE,
    rule_type VARCHAR(50) NOT NULL,
    is_enabled BOOLEAN DEFAULT TRUE,
    weight DOUBLE PRECISION NOT NULL DEFAULT 0.25,
    threshold DOUBLE PRECISION NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fraud_rules_rule_name ON fraud_rules(rule_name);
CREATE INDEX idx_fraud_rules_is_enabled ON fraud_rules(is_enabled);

-- Fraud alerts for manual review
CREATE TABLE IF NOT EXISTS fraud_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL,
    merchant_id UUID NOT NULL,
    risk_score DOUBLE PRECISION NOT NULL,
    alert_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'reviewing', 'resolved', 'dismissed')),
    assigned_to UUID,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_fraud_alerts_transaction_id ON fraud_alerts(transaction_id);
CREATE INDEX idx_fraud_alerts_status ON fraud_alerts(status);
CREATE INDEX idx_fraud_alerts_created_at ON fraud_alerts(created_at);

-- Insert default fraud rules
INSERT INTO fraud_rules (rule_name, rule_type, weight, threshold, config) VALUES
    ('velocity_check', 'velocity', 0.25, 0.7, '{"max_transactions_per_hour": 10, "window_minutes": 60}'),
    ('amount_check', 'amount', 0.20, 0.7, '{"max_amount_per_hour": 1000000}'),
    ('bin_check', 'country_mismatch', 0.15, 0.5, '{"allowed_mismatches": 0}'),
    ('ip_check', 'country_mismatch', 0.15, 0.5, '{"allowed_mismatches": 0}'),
    ('device_check', 'device_fingerprint', 0.10, 0.6, '{"max_users_per_device": 3}'),
    ('email_check', 'email_reputation', 0.10, 0.5, '{"check_disposable": true, "min_account_age_hours": 24}'),
    ('address_check', 'avs_mismatch', 0.05, 0.3, '{"require_match": false}')
ON CONFLICT (rule_name) DO NOTHING;
