pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_operators;
mod m20240101_000002_create_principals;
mod m20240101_000003_create_api_keys;
mod m20240101_000004_create_pending_changes;
mod m20240101_000005_create_kyb_cases;
mod m20240101_000006_create_outbox;
mod m20240101_000007_create_audit_log;
mod m20240101_000008_create_refresh_tokens;
mod m20240101_000009_create_event_store;
mod m20240101_000010_create_payment_intents;
mod m20240101_000011_create_routing_policies;
mod m20240101_000012_create_gateway_profiles;
mod m20240101_000013_create_invoices;
mod m20240101_000014_create_payment_links;
mod m20240101_000015_create_subscriptions;
mod m20240101_000016_create_disputes;
mod m20240101_000017_create_notifications;
mod m20240101_000018_create_risk_assessments;
mod m20240101_000019_create_settlement_batches;
mod m20240101_000020_create_documents;
mod m20240101_000021_create_ai_requests;
mod m20240101_000022_create_route_configs;
mod m20240101_000023_create_saga_instances;
mod m20240102_000001_add_row_level_security;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_operators::Migration),
            Box::new(m20240101_000002_create_principals::Migration),
            Box::new(m20240101_000003_create_api_keys::Migration),
            Box::new(m20240101_000004_create_pending_changes::Migration),
            Box::new(m20240101_000005_create_kyb_cases::Migration),
            Box::new(m20240101_000006_create_outbox::Migration),
            Box::new(m20240101_000007_create_audit_log::Migration),
            Box::new(m20240101_000008_create_refresh_tokens::Migration),
            Box::new(m20240101_000009_create_event_store::Migration),
            Box::new(m20240101_000010_create_payment_intents::Migration),
            Box::new(m20240101_000011_create_routing_policies::Migration),
            Box::new(m20240101_000012_create_gateway_profiles::Migration),
            Box::new(m20240101_000013_create_invoices::Migration),
            Box::new(m20240101_000014_create_payment_links::Migration),
            Box::new(m20240101_000015_create_subscriptions::Migration),
            Box::new(m20240101_000016_create_disputes::Migration),
            Box::new(m20240101_000017_create_notifications::Migration),
            Box::new(m20240101_000018_create_risk_assessments::Migration),
            Box::new(m20240101_000019_create_settlement_batches::Migration),
            Box::new(m20240101_000020_create_documents::Migration),
            Box::new(m20240101_000021_create_ai_requests::Migration),
            Box::new(m20240101_000022_create_route_configs::Migration),
            Box::new(m20240101_000023_create_saga_instances::Migration),
            Box::new(m20240102_000001_add_row_level_security::Migration),
        ]
    }
}
