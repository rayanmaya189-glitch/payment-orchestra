//! Database entities for SaaS Billing service.
//!
//! These are SeaORM entity models that map to database tables.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ─── SaaS Plan Entity ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "saas_plans")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub plan_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub price_monthly_minor: i64,
    pub price_per_txn_minor: i64,
    pub included_txns_monthly: i32,
    pub max_gateways: i32,
    pub max_team_members: i32,
    pub max_api_keys: i32,
    pub max_webhooks: i32,
    pub data_retention_days: i32,
    pub features: Json,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── Tenant Subscription Entity ──────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tenant_subscriptions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub plan_id: Uuid,
    pub status: String,
    pub current_period_start: DateTimeWithTimeZone,
    pub current_period_end: DateTimeWithTimeZone,
    pub trial_ends_at: Option<DateTimeWithTimeZone>,
    pub canceled_at: Option<DateTimeWithTimeZone>,
    pub cancel_reason: Option<String>,
    pub payment_method_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── Tenant Usage Entity ─────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tenant_usage")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub usage_id: Uuid,
    pub operator_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
    pub transaction_count: i32,
    pub transaction_volume_minor: i64,
    pub api_calls: i32,
    pub storage_bytes: i64,
    pub ai_queries: i32,
    pub overage_amount_minor: i64,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── SaaS Invoice Entity ─────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "saas_invoices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub subscription_id: Uuid,
    pub invoice_number: String,
    pub status: String,
    pub subtotal_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub currency: String,
    pub period_start: Date,
    pub period_end: Date,
    pub due_date: Date,
    pub paid_at: Option<DateTimeWithTimeZone>,
    pub line_items: Json,
    pub stripe_invoice_id: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── Team Member Entity ──────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tenant_team_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub invited_by: Uuid,
    pub invited_at: DateTimeWithTimeZone,
    pub accepted_at: Option<DateTimeWithTimeZone>,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── Audit Log Entity ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub log_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub old_value: Option<Json>,
    pub new_value: Option<Json>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
