//! SeaORM entity model definitions for event-sourced services.
//!
//! Each module maps to a PostgreSQL table. Nested complex types (Vec, enums)
//! are stored as JSONB columns. The struct must be named `Model` per SeaORM's
//! `DeriveEntityModel` macro convention — the table name is set via
//! `#[sea_orm(table_name = "...")]`.

pub mod subscription {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// `subscriptions` table entity.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "subscriptions")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub subscription_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub operator_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub customer_id: Uuid,
        pub plan_id: String,
        pub plan_amount_minor_units: i64,
        pub currency: String,
        pub status: String,
        pub current_period_start: DateTimeUtc,
        pub current_period_end: DateTimeUtc,
        pub billing_interval_days: i64,
        pub payment_method_token_id: Option<Uuid>,
        pub dunning_retry_count: i32,
        pub max_dunning_retries: i32,
        /// JSONB: serialized Vec<BillingCycle>
        pub billing_cycles: Json,
        /// JSONB: serialized Vec<DunningRetry>
        pub dunning_retries: Json,
        pub created_at: DateTimeUtc,
        pub cancelled_at: Option<DateTimeUtc>,
        pub paused_at: Option<DateTimeUtc>,
        pub resumed_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod invoice {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "invoices")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub invoice_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub operator_id: Uuid,
        pub order_reference: String,
        pub status: String,
        /// JSONB: serialized Vec<InvoiceLineItem>
        pub line_items: Json,
        pub total_amount_minor: i64,
        pub paid_amount_minor: i64,
        pub currency: String,
        pub due_date: DateTimeUtc,
        pub recipient_email: Option<String>,
        pub payment_intent_ids: Json,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod chargeback_case {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "chargeback_cases")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub chargeback_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub operator_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub payment_intent_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub acquirer_link_id: Uuid,
        pub status: String,
        pub reason_code: String,
        pub amount_minor_units: i64,
        pub currency: String,
        pub received_at: DateTimeUtc,
        pub representment_deadline: DateTimeUtc,
        pub resolved_at: Option<DateTimeUtc>,
        pub outcome: Option<String>,
        pub resolution_note: Option<String>,
        /// JSONB: serialized Vec<RepresentmentSubmission>
        pub submissions: Json,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod settlement_batch {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "settlement_batches")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub settlement_batch_id: Uuid,
        pub batch_file_name: String,
        pub status: String,
        pub ingested_at: DateTimeUtc,
        pub total_transactions: i32,
        pub total_amount_minor: i64,
        pub currency: String,
        /// JSONB: serialized Vec<SettlementRecord>
        pub records: Json,
        pub file_checksum: String,
        pub matched_at: Option<DateTimeUtc>,
        pub quarantined_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod ledger_entry {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ledger_entries")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub entry_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub transaction_id: Uuid,
        pub entry_type: String,
        pub amount_minor: i64,
        pub currency: String,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod settlement_expectation {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "settlement_expectations")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub expectation_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub payment_intent_id: Uuid,
        pub expected_amount_minor: i64,
        pub currency: String,
        pub expected_settlement_date: DateTimeUtc,
        pub status: String,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod fee_variance {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "fee_variances")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub variance_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub payment_intent_id: Uuid,
        pub expected_fee_minor: i64,
        pub actual_fee_minor: i64,
        pub variance_amount_minor: i64,
        pub currency: String,
        pub reason: Option<String>,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod payment_intent {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "payment_intents")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub payment_intent_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub operator_id: Uuid,
        pub amount_minor_units: i64,
        pub currency: String,
        pub status: String,
        pub payment_method_type: String,
        pub captured_amount_minor: i64,
        pub refunded_amount_minor: i64,
        /// JSONB: serialized Vec<RoutingAttempt>
        pub routing_attempts: Json,
        pub metadata_json: Option<String>,
        pub error_message: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod routing_policy {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "routing_policies")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub routing_policy_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub operator_id: Uuid,
        pub name: String,
        pub status: String,
        /// JSONB: serialized Vec<RoutingRule>
        pub rules: Json,
        pub created_at: DateTimeUtc,
        pub activated_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod payment_method_token {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "payment_method_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, column_type = "Uuid")]
        pub token_id: Uuid,
        #[sea_orm(column_type = "Uuid")]
        pub operator_id: Uuid,
        pub token_status: String,
        pub payment_method_type: String,
        pub token_ref: String,
        pub created_at: DateTimeUtc,
        pub expires_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
