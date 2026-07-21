use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::LedgerEntry;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ledger_entry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub entry_type: String,
    pub debit_amount_minor_units: i64,
    pub credit_amount_minor_units: i64,
    pub currency: String,
    pub source_acquirer: String,
    pub fee_amount_minor_units: Option<i64>,
    pub fee_currency: Option<String>,
    pub reconciliation_batch_id: Option<Uuid>,
    pub reconciled: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> LedgerEntry {
        LedgerEntry {
            entry_id: self.entry_id,
            transaction_id: self.transaction_id,
            entry_type: self.entry_type.clone(),
            debit_amount_minor_units: self.debit_amount_minor_units,
            credit_amount_minor_units: self.credit_amount_minor_units,
            currency: self.currency.clone(),
            source_acquirer: self.source_acquirer.clone(),
            fee: self.fee_amount_minor_units.map(|amt| shared_types::Money {
                amount_minor_units: amt,
                currency: shared_types::CurrencyCode::new(
                    self.fee_currency.as_deref().unwrap_or("AED"),
                )
                .unwrap_or_else(|_| shared_types::CurrencyCode::new("AED").unwrap()),
            }),
            reconciliation_batch_id: self.reconciliation_batch_id,
            reconciled: self.reconciled,
            created_at: self.created_at.into(),
        }
    }
}

impl From<LedgerEntry> for ActiveModel {
    fn from(e: LedgerEntry) -> Self {
        Self {
            entry_id: sea_orm::Set(e.entry_id),
            transaction_id: sea_orm::Set(e.transaction_id),
            entry_type: sea_orm::Set(e.entry_type),
            debit_amount_minor_units: sea_orm::Set(e.debit_amount_minor_units),
            credit_amount_minor_units: sea_orm::Set(e.credit_amount_minor_units),
            currency: sea_orm::Set(e.currency),
            source_acquirer: sea_orm::Set(e.source_acquirer),
            fee_amount_minor_units: sea_orm::Set(e.fee.as_ref().map(|f| f.amount_minor_units)),
            fee_currency: sea_orm::Set(e.fee.as_ref().map(|f| f.currency.0.clone())),
            reconciliation_batch_id: sea_orm::Set(e.reconciliation_batch_id),
            reconciled: sea_orm::Set(e.reconciled),
            created_at: sea_orm::Set(e.created_at.into()),
        }
    }
}
