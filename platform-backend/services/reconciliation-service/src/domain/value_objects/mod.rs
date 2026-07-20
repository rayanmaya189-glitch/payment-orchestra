#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementStatus { Pending, Polled, Matched, Settled, Exception }
impl SettlementStatus { pub fn as_str(&self) -> &'static str { match self { Self::Pending => "pending", Self::Polled => "polled", Self::Matched => "matched", Self::Settled => "settled", Self::Exception => "exception" } } }
