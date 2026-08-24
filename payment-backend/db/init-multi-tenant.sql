-- =============================================================================
-- Multi-Tenant Database Initialization for Payment Orchestra
-- Each microservice gets its own database and user credentials.
-- =============================================================================

-- ── Tier 1: Core Command Services ──────────────────────────────────────────

CREATE DATABASE payment_orchestra;
GRANT ALL PRIVILEGES ON DATABASE payment_orchestra TO platform;

CREATE USER operator_svc WITH PASSWORD 'operator_svc_db_pass';
CREATE DATABASE operator_svc_db OWNER operator_svc;
GRANT ALL PRIVILEGES ON DATABASE operator_svc_db TO operator_svc;

CREATE USER iam_svc WITH PASSWORD 'iam_svc_db_pass';
CREATE DATABASE iam_svc_db OWNER iam_svc;
GRANT ALL PRIVILEGES ON DATABASE iam_svc_db TO iam_svc;

CREATE USER compliance_svc WITH PASSWORD 'compliance_svc_db_pass';
CREATE DATABASE compliance_svc_db OWNER compliance_svc;
GRANT ALL PRIVILEGES ON DATABASE compliance_svc_db TO compliance_svc;

CREATE USER merchant_acquirer_link_svc WITH PASSWORD 'merchant_acquirer_link_svc_db_pass';
CREATE DATABASE merchant_acquirer_link_svc_db OWNER merchant_acquirer_link_svc;
GRANT ALL PRIVILEGES ON DATABASE merchant_acquirer_link_svc_db TO merchant_acquirer_link_svc;

-- ── Tier 2: Event-Sourced Domain Services ─────────────────────────────────

CREATE USER orchestration_svc WITH PASSWORD 'orchestration_svc_db_pass';
CREATE DATABASE orchestration_svc_db OWNER orchestration_svc;
GRANT ALL PRIVILEGES ON DATABASE orchestration_svc_db TO orchestration_svc;

CREATE USER invoice_svc WITH PASSWORD 'invoice_svc_db_pass';
CREATE DATABASE invoice_svc_db OWNER invoice_svc;
GRANT ALL PRIVILEGES ON DATABASE invoice_svc_db TO invoice_svc;

CREATE USER subscription_svc WITH PASSWORD 'subscription_svc_db_pass';
CREATE DATABASE subscription_svc_db OWNER subscription_svc;
GRANT ALL PRIVILEGES ON DATABASE subscription_svc_db TO subscription_svc;

CREATE USER dispute_svc WITH PASSWORD 'dispute_svc_db_pass';
CREATE DATABASE dispute_svc_db OWNER dispute_svc;
GRANT ALL PRIVILEGES ON DATABASE dispute_svc_db TO dispute_svc;

CREATE USER reconciliation_svc WITH PASSWORD 'reconciliation_svc_db_pass';
CREATE DATABASE reconciliation_svc_db OWNER reconciliation_svc;
GRANT ALL PRIVILEGES ON DATABASE reconciliation_svc_db TO reconciliation_svc;

-- ── Tier 3: Supporting Domain Services ────────────────────────────────────

CREATE USER payment_link_svc WITH PASSWORD 'payment_link_svc_db_pass';
CREATE DATABASE payment_link_svc_db OWNER payment_link_svc;
GRANT ALL PRIVILEGES ON DATABASE payment_link_svc_db TO payment_link_svc;

CREATE USER risk_svc WITH PASSWORD 'risk_svc_db_pass';
CREATE DATABASE risk_svc_db OWNER risk_svc;
GRANT ALL PRIVILEGES ON DATABASE risk_svc_db TO risk_svc;

CREATE USER notification_svc WITH PASSWORD 'notification_svc_db_pass';
CREATE DATABASE notification_svc_db OWNER notification_svc;
GRANT ALL PRIVILEGES ON DATABASE notification_svc_db TO notification_svc;

CREATE USER document_svc WITH PASSWORD 'document_svc_db_pass';
CREATE DATABASE document_svc_db OWNER document_svc;
GRANT ALL PRIVILEGES ON DATABASE document_svc_db TO document_svc;

CREATE USER analytics_svc WITH PASSWORD 'analytics_svc_db_pass';
CREATE DATABASE analytics_svc_db OWNER analytics_svc;
GRANT ALL PRIVILEGES ON DATABASE analytics_svc_db TO analytics_svc;

CREATE USER ai_assistant_svc WITH PASSWORD 'ai_assistant_svc_db_pass';
CREATE DATABASE ai_assistant_svc_db OWNER ai_assistant_svc;
GRANT ALL PRIVILEGES ON DATABASE ai_assistant_svc_db TO ai_assistant_svc;

-- ── Tier 4: Infrastructure / Gateway Services ─────────────────────────────

CREATE USER connector_gateway_svc WITH PASSWORD 'connector_gateway_svc_db_pass';
CREATE DATABASE connector_gateway_svc_db OWNER connector_gateway_svc;
GRANT ALL PRIVILEGES ON DATABASE connector_gateway_svc_db TO connector_gateway_svc;

CREATE USER ai_gateway_svc WITH PASSWORD 'ai_gateway_svc_db_pass';
CREATE DATABASE ai_gateway_svc_db OWNER ai_gateway_svc;
GRANT ALL PRIVILEGES ON DATABASE ai_gateway_svc_db TO ai_gateway_svc;

CREATE USER api_gateway_svc WITH PASSWORD 'api_gateway_svc_db_pass';
CREATE DATABASE api_gateway_svc_db OWNER api_gateway_svc;
GRANT ALL PRIVILEGES ON DATABASE api_gateway_svc_db TO api_gateway_svc;

CREATE USER saga_coordinator_svc WITH PASSWORD 'saga_coordinator_svc_db_pass';
CREATE DATABASE saga_coordinator_svc_db OWNER saga_coordinator_svc;
GRANT ALL PRIVILEGES ON DATABASE saga_coordinator_svc_db TO saga_coordinator_svc;

CREATE USER scheduler_svc WITH PASSWORD 'scheduler_svc_db_pass';
CREATE DATABASE scheduler_svc_db OWNER scheduler_svc;
GRANT ALL PRIVILEGES ON DATABASE scheduler_svc_db TO scheduler_svc;

CREATE USER outbox_relay_svc WITH PASSWORD 'outbox_relay_svc_db_pass';
CREATE DATABASE outbox_relay_svc_db OWNER outbox_relay_svc;
GRANT ALL PRIVILEGES ON DATABASE outbox_relay_svc_db TO outbox_relay_svc;

CREATE USER merchant_connector_onboarding_svc WITH PASSWORD 'merchant_connector_onboarding_svc_db_pass';
CREATE DATABASE merchant_connector_onboarding_svc_db OWNER merchant_connector_onboarding_svc;
GRANT ALL PRIVILEGES ON DATABASE merchant_connector_onboarding_svc_db TO merchant_connector_onboarding_svc;
